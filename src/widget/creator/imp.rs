use super::UnitCreatorWindow;
use crate::{
    upgrade,
    widget::{
        app_window::AppWindow,
        close_window_shortcut,
        creator::{
            ACTION_CREATOR_NEXT, ACTION_CREATOR_PREVIOUS, ACTION_CREATOR_UNIT_BUS, UnitCreateType,
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
use tracing::{error, warn};

// const PROPERTY_NAME: &str = "creation-type";
const PAGE_FIRST: &str = "first-page";
const PAGE_LAUNCH: &str = "launch-page";
const PAGE_TIMER: &str = "timer-page";
const PAGE_SERVICE: &str = "service-page";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub(super) enum PageType {
    #[default]
    Start,
    Service,
    Timer,
    Launch,
}

impl PageType {
    fn next(&self, creation_type: UnitCreateType) -> Option<&'static str> {
        match (self, creation_type) {
            (PageType::Start, UnitCreateType::Timer) => Some(PAGE_TIMER),
            (PageType::Start, _) => Some(PAGE_SERVICE),
            (PageType::Service, UnitCreateType::TimerService) => Some(PAGE_TIMER),
            (PageType::Service, _) => Some(PAGE_LAUNCH),
            (PageType::Timer, _) => Some(PAGE_LAUNCH),
            (PageType::Launch, _) => None,
        }
    }
}

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
    nav_row: TemplateChild<NavigationRow>,

    pub(super) app_window: OnceCell<AppWindow>,
    pub timer_page: OnceCell<TimerCreatorPage>,

    #[property(get, set=Self::set_creation_unit_type, default)]
    pub(super) creation_type: Cell<UnitCreateType>,

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

    /* fn service_page(&self) {
        if let Some(widget) = self.sections.borrow().get(&PageType::Service) {
            if widget.parent().is_none() {
                // self.carousel.append(widget);
            }
            // widget.set_property(PROPERTY_NAME, unit_type);
        } else {
            let service_page = ServiceCreatorPage::new(self.obj().downgrade());
            let unit_file_page = UnitFileCreatorPage::new();
            let service_navigation = adw::NavigationView::new();

            //The push add is important , case if 2 adds the navigation stamer
            service_navigation.push(&service_page);
            service_navigation.add(&unit_file_page);

            let unit_file_page = unit_file_page.downgrade();
            let service_page = service_page.downgrade();
            service_navigation.connect_visible_page_notify(move |nav| {
                match nav.visible_page_tag().as_deref() {
                    Some("service_base") => {
                        let unit_file_page = upgrade!(unit_file_page);
                        let service_page = upgrade!(service_page);
                        let text = unit_file_page.file_text();

                        service_page.update_file_data(&text);
                    }
                    Some("unit_file_page") => {
                        let unit_file_page = upgrade!(unit_file_page);
                        let service_page = upgrade!(service_page);
                        service_page.update_view(&unit_file_page);
                    }
                    Some(visible_page) => warn!("Service page notify page {:?}", visible_page),
                    None => warn!("Service page notify page None"),
                }
            });

            // service_page.set_property(PROPERTY_NAME, unit_type);
        }
    } */

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
}

#[glib::derived_properties]
impl ObjectImpl for UnitCreatorWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
        close_window_shortcut(self.obj().as_ref());

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

        let previous: gio::ActionEntry<_> = {
            let navigation = self.navigation.clone();
            // let creation_window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_PREVIOUS[8..])
                .activate(move |_, _, _| {
                    navigation.pop();
                })
                .build()
        };

        self.action_group
            .borrow()
            .add_action_entries([next, previous]);

        // let s = SimpleActionGroup::new();
        let first_page = UnitCreatorFirstPage::new(self.obj().downgrade(), PAGE_FIRST);
        let end_page = LaunchCreatorPage::new(self.obj().downgrade(), PAGE_LAUNCH);
        let timer_page = TimerCreatorPage::new(self.obj().downgrade(), PAGE_TIMER);
        let service_page = ServiceCreatorPage::new(self.obj().downgrade(), PAGE_SERVICE);
        let unit_file_page = UnitFileCreatorPage::new();

        self.navigation.push(&first_page);
        self.navigation.add(&end_page);
        self.navigation.add(&timer_page);
        self.navigation.add(&service_page);
        self.navigation.add(&unit_file_page);

        let window = self.obj().downgrade();
        self.navigation.connect_visible_page_notify(move |nav| {
            let window = upgrade!(window);
            match nav.visible_page_tag().as_deref() {
                Some(PAGE_FIRST) => window.imp().page_type.set(PageType::Start),
                Some(PAGE_TIMER) => window.imp().page_type.set(PageType::Timer),
                Some(PAGE_SERVICE) => window.imp().page_type.set(PageType::Service),
                Some(PAGE_LAUNCH) => window.imp().page_type.set(PageType::Launch),
                Some(a) => {
                    warn!("unknown {a:?}")
                }
                None => warn!("None"),
            }
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
