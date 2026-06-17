use super::UnitCreatorWindow;
use crate::{
    format2,
    systemd_gui::new_settings,
    upgrade,
    widget::{
        app_window::AppWindow,
        close_window_shortcut_no_escape,
        creator::{
            ACTION_CREATOR_CREATE, ACTION_CREATOR_FILE, ACTION_CREATOR_NEXT,
            ACTION_CREATOR_PREVIOUS, ACTION_CREATOR_UNIT_BUS, PageType, SaveUnit, UnitCreateType,
            first_page::UnitCreatorFirstPage, launch_creator_page::LaunchCreatorPage,
            navigation_row::NavigationRow, service_creator_page::ServiceCreatorPage,
            timer_creator_page::TimerCreatorPage, unit_file_creator_page::UnitFileCreatorPage,
        },
        replace_tags,
    },
};
use adw::prelude::*;
use adw::subclass::window::AdwWindowImpl;
use base::enums::UnitDBusLevel;
use gettextrs::pgettext;
use gio::{SimpleActionGroup, prelude::ActionMapExtManual};
use gtk::{TemplateChild, gio, glib, subclass::prelude::*};
use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, Ref, RefCell},
    collections::HashSet,
    path::PathBuf,
};
use systemd::errors::SystemdErrors;
use tracing::{error, info, warn};

// const PROPERTY_NAME: &str = "creation-type";

const WINDOW_SIZE: &str = "create-unit-window-size";
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
    start_page: OnceCell<UnitCreatorFirstPage>,
    timer_page: OnceCell<TimerCreatorPage>,
    service_page: OnceCell<ServiceCreatorPage>,
    first_page: OnceCell<UnitCreatorFirstPage>,
    last_page: OnceCell<LaunchCreatorPage>,

    #[property(get, set=Self::set_creation_unit_type, default)]
    pub(super) creation_type: Cell<UnitCreateType>,

    #[property(get, set=Self::set_page_type, default)]
    pub(super) page_type: Cell<PageType>,

    #[property(get, set=Self::set_bus_level, default)]
    pub(super) level: Cell<UnitDBusLevel>,

    pub(super) system_file_list: RefCell<HashSet<String>>,
    pub(super) session_file_list: RefCell<HashSet<String>>,

    pub(super) system_file_list_model: RefCell<gtk::StringList>,
    pub(super) session_file_list_model: RefCell<gtk::StringList>,

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
        // klass.bind_template_callbacks();
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

        let valid = match self.page_type.get() {
            PageType::Start if let Some(page) = self.start_page.get() => page.validate(),
            PageType::Service => true,
            PageType::Timer => true,
            _ => true,
        };

        if valid {
            self.page_type.get().next(creation_type)
        } else {
            None
        }
    }

    fn set_page_type(&self, page: PageType) {
        self.page_type.set(page);
        self.nav_row.set_page_type(page, self.creation_type.get());
    }

    pub fn set_bus_level(&self, level: UnitDBusLevel) {
        self.level.set(level);
        let creation_window = self.obj().clone();
        glib::spawn_future_local(async move {
            creation_window.imp().fill_unit_files().await;
        });
    }

    async fn fill_unit_files(&self) {
        let level = self.level.get();
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
                let (mut set, model) = match level {
                    UnitDBusLevel::System | UnitDBusLevel::Both => (
                        self.system_file_list.borrow_mut(),
                        self.system_file_list_model.borrow().clone(),
                    ),
                    UnitDBusLevel::UserSession => (
                        self.session_file_list.borrow_mut(),
                        self.session_file_list_model.borrow().clone(),
                    ),
                };

                for ufile in list.into_iter() {
                    let value = ufile.unit_primary_name();
                    set.insert(value.to_owned());
                }

                let mut vec = set.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
                vec.push(""); //for unselect
                vec.sort();

                model.splice(0, model.n_items(), &vec);
            }
            Ok(_) => {
                warn!("unreachable");
            }
            Err(err) => warn!("List unit {:?}", err),
        };

        if let Some(timer_page) = self.timer_page.get() {
            timer_page.update_from_unit_info();
        }

        if let Some(service_page) = self.service_page.get() {
            service_page.update_from_unit_info();
        }
    }

    pub fn get_trigger_units(&self) -> Ref<'_, HashSet<String>> {
        let level = self.level.get();

        match level {
            UnitDBusLevel::System | UnitDBusLevel::Both => self.system_file_list.borrow(),
            UnitDBusLevel::UserSession => self.session_file_list.borrow(),
        }
    }

    pub fn get_trigger_units_model(&self) -> gtk::StringList {
        let level = self.level.get();

        match level {
            UnitDBusLevel::System | UnitDBusLevel::Both => {
                self.system_file_list_model.borrow().clone()
            }
            UnitDBusLevel::UserSession => self.session_file_list_model.borrow().clone(),
        }
    }

    pub fn service_file_path(&self) -> Option<PathBuf> {
        self.file_path("service")
    }

    pub fn timer_file_path(&self) -> Option<PathBuf> {
        self.file_path("timer")
    }

    fn file_path(&self, suffix: &str) -> Option<PathBuf> {
        let Some(first_page) = self.first_page.get() else {
            error!("first page None");
            return None;
        };

        let (runtime, prefix) = first_page.fetch_settings();
        let user_session = self.level.get().user_session();
        let Ok(dir) = base::file::determine_unit_file_path_dir(runtime, user_session)
            .inspect_err(|err| error!("path error {err:?}"))
        else {
            return None;
        };

        Some(dir.join(prefix).with_extension(suffix))
    }

    pub fn service_unit_name(&self) -> Option<String> {
        self.unit_name("service")
    }

    pub fn timer_unit_name(&self) -> Option<String> {
        self.unit_name("timer")
    }

    fn unit_name(&self, suffix: &str) -> Option<String> {
        let Some(first_page) = self.first_page.get() else {
            error!("first page None");
            return None;
        };

        let (_, prefix) = first_page.fetch_settings();

        Some(format!("{prefix}.{suffix}"))
    }

    fn save_unit_files(&self) {
        let file_contents = match self.creation_type.get() {
            UnitCreateType::Service => {
                if let Some(service_page) = self.service_page.get() {
                    let Some(file_path) = self.service_file_path() else {
                        error!("No file path");
                        return;
                    };
                    let content = service_page.file_content();
                    vec![(file_path, content)]
                } else {
                    Vec::new()
                }
            }
            UnitCreateType::Timer => {
                if let Some(timer_page) = self.timer_page.get() {
                    let Some(file_path) = self.timer_file_path() else {
                        error!("No file path");
                        return;
                    };
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
                    let Some(service_file_path) = self.service_file_path() else {
                        error!("No file path");
                        return;
                    };
                    let content_s = service_page.file_content();

                    let Some(file_path) = self.timer_file_path() else {
                        error!("No file path");
                        return;
                    };
                    let content = timer_page.file_content();
                    vec![(service_file_path, content_s), (file_path, content)]
                } else {
                    Vec::new()
                }
            }
        };

        let window = self.obj().clone();
        let user_session = self.level.get().user_session();
        glib::spawn_future_local(async move {
            let (sender, receiver) = tokio::sync::oneshot::channel();

            systemd::runtime().spawn(async move {
                let mut response = Err(SystemdErrors::Custom("No file to save".to_string()));

                for (file_path, content) in file_contents {
                    response =
                        systemd::create_file(user_session, &file_path.to_string_lossy(), &content)
                            .await;

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

    fn handle_create_after(&self, message: SaveUnit) {
        match message {
            SaveUnit::Created => {
                let unit_name = self.created_unit_name().join(" &amp; ");
                let msg = pgettext("create", "Unit {} Created!");
                let msg = format2!(msg, format!("<unit>{unit_name}</unit>"));
                self.add_toast_message(&msg, true, None);
            }

            SaveUnit::CreateError(ref systemd_errors) => {
                let human_error = systemd_errors.human_error_type();
                let msg = pgettext("create", "Creation Failed! {}");
                let msg = format2!(msg, format!("<red>{human_error}</red>"));
                self.add_toast_message(&msg, true, None);
            }
        }

        if let Some(last_page) = self.last_page.get() {
            last_page.handle_create_after(message);
        }
    }

    pub fn save_window_context(&self) -> Result<(), glib::BoolError> {
        let size = self.obj().default_size().to_variant();

        let settings = new_settings();

        settings.set_value(WINDOW_SIZE, &size)?;

        Ok(())
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = new_settings();

        let size = settings.value(WINDOW_SIZE);

        let (width, height) = size.get::<(i32, i32)>().unwrap();

        // Set the size of the window
        self.obj().set_default_size(width, height);
    }

    pub(super) fn add_toast_message(
        &self,
        message: &str,
        use_markup: bool,
        action: Option<(&str, String, bool)>,
    ) {
        let msg = if use_markup {
            let out = replace_tags(message);
            Cow::from(out)
        } else {
            Cow::from(message)
        };

        let toast = adw::Toast::builder()
            .title(msg)
            .use_markup(use_markup)
            .build();

        if let Some((action_name, ref button_label, user_session)) = action {
            info!("Toast action {:?} user_session {user_session}", action);
            toast.set_action_name(Some(action_name));
            toast.set_action_target_value(Some(&user_session.to_variant()));
            toast.set_button_label(Some(button_label));
        }

        self.toast_overlay.add_toast(toast)
    }

    fn created_unit_name(&self) -> Vec<String> {
        let Some(first_page) = self.first_page.get() else {
            error!("first page None");
            return Vec::default();
        };

        let (_, prefix) = first_page.fetch_settings();

        let suffixes = match self.creation_type.get() {
            UnitCreateType::Service => vec!["service"],
            UnitCreateType::Timer => vec!["timer"],
            UnitCreateType::TimerService => vec!["timer", "service"],
        };

        suffixes
            .iter()
            .map(|suffix| format!("{prefix}.{suffix}"))
            .collect()
    }
}

#[glib::derived_properties]
impl ObjectImpl for UnitCreatorWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
        close_window_shortcut_no_escape(self.obj().as_ref());
        self.set_page_type(PageType::Start);

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
        let last_page = LaunchCreatorPage::new(self.obj().downgrade(), PageType::Launch);
        let timer_page = TimerCreatorPage::new(self.obj().downgrade(), PageType::Timer);
        let service_page = ServiceCreatorPage::new(self.obj().downgrade(), PageType::Service);
        let timer_file_page = UnitFileCreatorPage::new(PageType::TimerFile);
        let service_file_page = UnitFileCreatorPage::new(PageType::ServiceFile);

        self.navigation.push(&first_page);
        self.navigation.add(&last_page);
        self.navigation.add(&timer_page);
        self.navigation.add(&service_page);
        self.navigation.add(&service_file_page);
        self.navigation.add(&timer_file_page);

        let _ = self.start_page.set(first_page.clone());
        let _ = self.timer_page.set(timer_page.clone());
        let _ = self.service_page.set(service_page.clone());
        let _ = self.first_page.set(first_page.clone());
        let _ = self.last_page.set(last_page.clone());
        let window = self.obj().downgrade();
        let service_page = service_page.downgrade();
        let service_file_page = service_file_page.downgrade();
        let timer_page = timer_page.downgrade();
        let timer_file_page = timer_file_page.downgrade();
        let last_page = last_page.downgrade();

        self.navigation.connect_visible_page_notify(move |nav| {
            let window = upgrade!(window);
            let new_page: PageType = nav.visible_page_tag().as_deref().into();

            match (new_page, window.page_type()) {
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
                (PageType::Service, PageType::Start | PageType::Launch) => {}
                (_, PageType::ServiceFile) => {
                    let service_file_page = upgrade!(service_file_page);
                    let service_page = upgrade!(service_page);
                    let text = service_file_page.file_text();
                    service_page.update_from_file_content(&text);
                }
                (PageType::Timer, _) => {
                    let timer_page = upgrade!(timer_page);
                    timer_page.set_view(window.creation_type());
                }
                (_, PageType::TimerFile) => {
                    let timer_file_page = upgrade!(timer_file_page);
                    let timer_page = upgrade!(timer_page);
                    let text = timer_file_page.file_text();
                    timer_page.update_from_file_content(&text);
                }
                (PageType::Launch, _) => {
                    let last_page = upgrade!(last_page);
                    last_page.update_page();
                }
                _ => {}
            }
            window.set_page_type(new_page);
        });

        self.load_window_size();

        let window = self.obj().clone();
        glib::spawn_future_local(async move {
            if let Err(err) = systemd::test_flatpak_spawn() {
                warn!("Flatpak Spawn fail {err:?}");
                window.imp().banner.set_revealed(true);
            }
        });
    }
}

impl WidgetImpl for UnitCreatorWindowImp {}

impl WindowImpl for UnitCreatorWindowImp {
    fn close_request(&self) -> glib::Propagation {
        if let Err(err) = self.save_window_context() {
            error!("Failed to save window state {:?}", err);
        }

        self.parent_close_request();
        glib::Propagation::Proceed
    }
}

impl AdwWindowImpl for UnitCreatorWindowImp {}
