use super::UnitCreatorWindow;
use crate::{
    upgrade,
    widget::{
        app_window::AppWindow,
        close_window_shortcut,
        creator::{
            ACTION_CREATOR_CREATE, ACTION_CREATOR_FILE, ACTION_CREATOR_NEXT,
            ACTION_CREATOR_PREVIOUS, ACTION_CREATOR_UNIT_BUS, PageType, UnitCreateType,
            first_page::UnitCreatorFirstPage, launch_creator_page::LaunchCreatorPage,
            navigation_row::NavigationRow, service_creator_page::ServiceCreatorPage,
            timer_creator_page::TimerCreatorPage, unit_file_creator_page::UnitFileCreatorPage,
        },
    },
};
use adw::prelude::*;
use adw::subclass::window::AdwWindowImpl;
use base::enums::UnitDBusLevel;
use gio::{SimpleActionGroup, prelude::ActionMapExtManual};
use gtk::{TemplateChild, gio, glib, subclass::prelude::*};
use std::{
    cell::{Cell, OnceCell, Ref, RefCell},
    collections::HashSet,
};
use systemd::errors::SystemdErrors;
use tracing::{error, warn};

// const PROPERTY_NAME: &str = "creation-type";

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator.ui")]
#[properties(wrapper_type = super::UnitCreatorWindow)]
pub struct UnitCreatorWindowImp {
    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,

    #[template_child]
    navigation: TemplateChild<adw::NavigationView>,

    #[template_child]
    banner: TemplateChild<adw::Banner>,

    #[template_child]
    toast_overlay: TemplateChild<adw::ToastOverlay>,

    #[template_child]
    nav_row: TemplateChild<NavigationRow>,

    pub(super) app_window: OnceCell<AppWindow>,
    timer_page: OnceCell<TimerCreatorPage>,
    service_page: OnceCell<ServiceCreatorPage>,
    first_page: OnceCell<UnitCreatorFirstPage>,

    #[property(get, set=Self::set_creation_unit_type, default)]
    pub(super) creation_type: Cell<UnitCreateType>,

    #[property(get, set=Self::set_page, default)]
    pub(super) page_type: Cell<PageType>,

    pub(super) bus_level: Cell<UnitDBusLevel>,

    pub(super) system_file_list: RefCell<HashSet<String>>,
    pub(super) session_file_list: RefCell<HashSet<String>>,

    pub(super) action_group: RefCell<SimpleActionGroup>,
}

#[glib::object_subclass]
impl ObjectSubclass for UnitCreatorWindowImp {
    const NAME: &'static str = "UnitCreatorWindow";
    type Type = UnitCreatorWindow;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        // let _ = NavigationRow::new();
        klass.bind_template();
        //klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl UnitCreatorWindowImp {
    pub(super) fn set_creation_unit_type(&self, unit_type: UnitCreateType) {
        self.creation_type.set(unit_type);
        // self.insert_page(&unit_type);
        self.window_title.set_subtitle(&unit_type.title());
    }

    fn next(&self) -> Option<&'static str> {
        let creation_type = self.creation_type.get();
        self.page_type.get().next(creation_type)
    }

    fn set_page(&self, page: PageType) {
        self.page_type.set(page);
        self.nav_row.set_page_type(page, self.creation_type.get());
    }

    pub async fn fill_unit_files(&self) {
        let level = self.bus_level.get();
        {
            let set = match level {
                UnitDBusLevel::System | UnitDBusLevel::Both => self.system_file_list.borrow(),
                UnitDBusLevel::UserSession => self.session_file_list.borrow(),
            };
            if !set.is_empty() {
                return;
            }
        }

        let (sender, receiver) = tokio::sync::oneshot::channel();
        crate::systemd::runtime().spawn(async move {
            let response = systemd::list_unit_files(level).await;
            if let Err(e) = sender.send(response) {
                error!("Channel closed unexpectedly: {e:?}");
            }
        });

        let Ok(response) = receiver.await else {
            error!("Tokio channel dropped");
            return;
        };

        match response {
            Ok(systemd::ListUnitResponse::File(_, list)) => {
                let mut set = match level {
                    UnitDBusLevel::System | UnitDBusLevel::Both => {
                        self.system_file_list.borrow_mut()
                    }
                    UnitDBusLevel::UserSession => self.session_file_list.borrow_mut(),
                };
                for ufile in list {
                    set.insert(ufile.unit_primary_name().to_owned());
                }
            }
            Ok(_) => {
                warn!("unreachable");
            }
            Err(err) => warn!("List unit {:?}", err),
        };

        if let Some(timer_page) = self.timer_page.get() {
            timer_page.update_from_unit_info();
        }
    }

    pub fn get_trigger_units(&self) -> Ref<'_, HashSet<String>> {
        let level = self.bus_level.get();

        match level {
            UnitDBusLevel::System | UnitDBusLevel::Both => self.system_file_list.borrow(),
            UnitDBusLevel::UserSession => self.session_file_list.borrow(),
        }
    }

    fn save_unit_files(&self) {
        let Some(first_page) = self.first_page.get() else {
            error!("first page None");
            return;
        };

        let (runtime, prefix) = first_page.fetch_settings();
        let user_session = self.bus_level.get().user_session();
        let Ok(dir) = base::file::determine_unit_file_path_dir(runtime, user_session)
            .inspect_err(|err| error!("path error {err:?}"))
        else {
            return;
        };

        let file_contents = match self.creation_type.get() {
            UnitCreateType::Service => {
                if let Some(service_page) = self.service_page.get() {
                    let file_path = format!("{dir}{prefix}.service");
                    let content = service_page.file_content();
                    vec![(file_path, content)]
                } else {
                    Vec::new()
                }
            }
            UnitCreateType::Timer => {
                if let Some(timer_page) = self.timer_page.get() {
                    let file_path = format!("{dir}{prefix}.timer");
                    let content = timer_page.file_content();
                    vec![(file_path, content)]
                } else {
                    Vec::new()
                }
            }
            UnitCreateType::TimerService => {
                if let Some(service_page) = self.service_page.get()
                    && let Some(timer_page) = self.timer_page.get()
                {
                    let file_path_s = format!("{dir}{prefix}.service");
                    let content_s = service_page.file_content();

                    let file_path = format!("{dir}{prefix}.timer");
                    let content = timer_page.file_content();
                    vec![(file_path_s, content_s), (file_path, content)]
                } else {
                    Vec::new()
                }
            }
        };

        let window = self.obj().clone();
        glib::spawn_future_local(async move {
            let (sender, receiver) = tokio::sync::oneshot::channel();

            systemd::runtime().spawn(async move {
                let mut response = Err(SystemdErrors::Custom("No file to save".to_string()));

                for (file_path, content) in file_contents {
                    response = systemd::create_file(user_session, &file_path, &content).await;

                    if response.is_err() {
                        break;
                    }
                }

                if let Err(e) = sender.send(response) {
                    error!("Channel closed unexpectedly: {e:?}");
                }
            });

            let Ok(response) = receiver.await else {
                error!("Tokio channel dropped");
                return;
            };

            let msg = match response {
                Ok(_) => SaveUnit::Created,
                Err(err) => {
                    warn!("Create Unit Error {err:?}");
                    SaveUnit::CreateError(err)
                }
            };
            window.imp().handle_create_after(msg);
        });
    }

    fn handle_create_after(&self, msg: SaveUnit) {
        let toast = match msg {
            SaveUnit::Created => adw::Toast::builder()
                .use_markup(true)
                .title("Unit Created")
                .build(),

            SaveUnit::CreateError(systemd_errors) => adw::Toast::builder()
                .use_markup(true)
                .title(systemd_errors.human_error_type())
                .build(),
        };
        self.toast_overlay.add_toast(toast);
    }
}

enum SaveUnit {
    Created,
    CreateError(SystemdErrors),
}

#[glib::derived_properties]
impl ObjectImpl for UnitCreatorWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
        close_window_shortcut(self.obj().as_ref());
        self.set_page(PageType::Start);

        self.banner.set_use_markup(true);
        self.banner.set_css_classes(&["warning", "construction"]);

        self.obj().insert_action_group(
            &ACTION_CREATOR_UNIT_BUS[0..7],
            Some(&self.action_group.borrow().clone()),
        );

        let next: gio::ActionEntry<_> = {
            let window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_NEXT[8..])
                .activate(move |_, _, _| {
                    let window = upgrade!(window);
                    if let Some(next) = window.imp().next() {
                        window.imp().navigation.push_by_tag(next);
                    }
                })
                .build()
        };
        let file: gio::ActionEntry<_> = {
            let window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_FILE[8..])
                .activate(move |_, _, _| {
                    let window = upgrade!(window);
                    match window.page_type() {
                        PageType::Service => window
                            .imp()
                            .navigation
                            .push_by_tag(PageType::ServiceFile.id()),

                        PageType::Timer => window
                            .imp()
                            .navigation
                            .push_by_tag(PageType::TimerFile.id()),
                        _ => {}
                    }
                })
                .build()
        };

        let previous: gio::ActionEntry<_> = {
            let navigation = self.navigation.clone();
            // let creation_window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_PREVIOUS[8..])
                .activate(move |_, _, _| {
                    navigation.pop();
                })
                .build()
        };

        let create: gio::ActionEntry<_> = {
            let window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_CREATE[8..])
                .activate(move |_, _, _| {
                    let window = upgrade!(window);
                    window.imp().save_unit_files();
                })
                .build()
        };

        self.action_group
            .borrow()
            .add_action_entries([next, previous, file, create]);

        // let s = SimpleActionGroup::new();
        let first_page = UnitCreatorFirstPage::new(self.obj().downgrade(), PageType::Start);
        let end_page = LaunchCreatorPage::new(self.obj().downgrade(), PageType::Launch);
        let timer_page = TimerCreatorPage::new(self.obj().downgrade(), PageType::Timer);
        let service_page = ServiceCreatorPage::new(self.obj().downgrade(), PageType::Service);
        let timer_file_page = UnitFileCreatorPage::new(PageType::TimerFile);
        let service_file_page = UnitFileCreatorPage::new(PageType::ServiceFile);

        self.navigation.push(&first_page);
        self.navigation.add(&end_page);
        self.navigation.add(&timer_page);
        self.navigation.add(&service_page);
        self.navigation.add(&service_file_page);
        self.navigation.add(&timer_file_page);

        let _ = self.timer_page.set(timer_page.clone());
        let _ = self.service_page.set(service_page.clone());
        let _ = self.first_page.set(first_page.clone());
        let window = self.obj().downgrade();
        let service_page = service_page.downgrade();
        let service_file_page = service_file_page.downgrade();
        let timer_page = timer_page.downgrade();
        let timer_file_page = timer_file_page.downgrade();
        self.navigation.connect_visible_page_notify(move |nav| {
            let window = upgrade!(window);
            let page: PageType = nav.visible_page_tag().as_deref().into();

            window.set_page_type(page);

            match (page, window.page_type()) {
                (PageType::ServiceFile, _) => {
                    let service_file_page = upgrade!(service_file_page);
                    let service_page = upgrade!(service_page);
                    service_page.update_view(&service_file_page);
                }
                (PageType::TimerFile, _) => {
                    let timer_file_page = upgrade!(timer_file_page);
                    let timer_page = upgrade!(timer_page);
                    timer_page.update_view(&timer_file_page);
                }
                (PageType::Service, PageType::ServiceFile) => {
                    let service_file_page = upgrade!(service_file_page);
                    let service_page = upgrade!(service_page);
                    let text = service_file_page.file_text();
                    service_page.update_file_data(&text);
                }
                (PageType::Timer, PageType::TimerFile) => {
                    let timer_file_page = upgrade!(timer_file_page);
                    let timer_page = upgrade!(timer_page);
                    let text = timer_file_page.file_text();
                    timer_page.update_file_data(&text);
                }
                _ => {}
            }
            window.set_page_type(page);
        });
    }
}

impl WidgetImpl for UnitCreatorWindowImp {}

impl WindowImpl for UnitCreatorWindowImp {
    fn close_request(&self) -> glib::Propagation {
        self.parent_close_request();
        glib::Propagation::Proceed
    }
}

impl AdwWindowImpl for UnitCreatorWindowImp {}
