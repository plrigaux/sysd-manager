// const WINDOW_HEIGHT: &str = "unit-creator-window-height";
// const WINDOW_WIDTH: &str = "unit-creator-window-width";
use super::UnitCreatorWindow;
use crate::{
    systemd_gui::clear_on_escape,
    upgrade,
    widget::{
        app_window::AppWindow,
        creator::{
            UnitCreateType, service_creator_page::ServiceCreatorPage,
            timer_creator_page::TimerCreatorPage,
        },
    },
};
use adw::prelude::*;
use adw::subclass::window::AdwWindowImpl;
use base::enums::UnitDBusLevel;
use gio::{SimpleActionGroup, prelude::ActionMapExtManual};
use glib::variant::ToVariant;
use gtk::{
    glib::{self},
    subclass::prelude::*,
};
use regex::Regex;
use std::{
    cell::{Cell, OnceCell, Ref, RefCell},
    collections::{HashMap, HashSet},
};
use tracing::{debug, warn};

const PROPERTY_NAME: &str = "creation-type";
const VALID_UNIT_NAME: &str = r"^[a-zA-Z0-9._:\-]+@?$";
const ACTION_CREATOR_UNIT_BUS: &str = "creator.unit_bus_selection";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator.ui")]
pub struct UnitCreatorWindowImp {
    #[template_child]
    carousel: TemplateChild<adw::Carousel>,

    #[template_child]
    unit_name_prefix: TemplateChild<adw::EntryRow>,

    section: RefCell<HashMap<UnitCreateType, gtk::Widget>>,

    pub(super) app_window: OnceCell<AppWindow>,

    creation_type: Cell<UnitCreateType>,
    level: Cell<UnitDBusLevel>,
    system_file_list: RefCell<HashSet<String>>,
    session_file_list: RefCell<HashSet<String>>,
    action_group: RefCell<SimpleActionGroup>,
    re: OnceCell<Regex>,
}

#[glib::object_subclass]
impl ObjectSubclass for UnitCreatorWindowImp {
    const NAME: &'static str = "UnitCreatorWindow";
    type Type = UnitCreatorWindow;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        //klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl UnitCreatorWindowImp {
    fn set_unit_type(&self, unit_type: &glib::Variant) {
        let unit_type: UnitCreateType = unit_type.into();
        self.creation_type.set(unit_type);
        match unit_type {
            UnitCreateType::Service => {
                self.insert_page(&unit_type);
            }
            UnitCreateType::Timer => {
                self.insert_page(&unit_type);
            }
            UnitCreateType::TimerService => {
                self.insert_page(&unit_type);
            }
            UnitCreateType::Unknown => {}
        }
    }

    fn insert_page(&self, unit_type: &UnitCreateType) {
        for (creation_type, widget) in self.section.borrow().iter() {
            match (unit_type, creation_type) {
                (UnitCreateType::Service, UnitCreateType::Timer) => {
                    self.remove_from_carousel(widget)
                }
                (UnitCreateType::Service, _) => {}
                (UnitCreateType::Timer, UnitCreateType::Service) => {
                    self.remove_from_carousel(widget)
                }
                (UnitCreateType::Timer, _) => {}
                (UnitCreateType::TimerService, _) => self.remove_from_carousel(widget),
                (_, _) => {}
            }
        }

        match unit_type {
            UnitCreateType::Service => {
                self.service_page(unit_type);
            }
            UnitCreateType::Timer => {
                self.timer_page(unit_type);
            }
            UnitCreateType::TimerService => {
                self.service_page(unit_type);

                self.timer_page(unit_type);
            }
            _ => {}
        };
    }

    fn remove_from_carousel(&self, w: &gtk::Widget) {
        if w.parent().is_some() {
            self.carousel.remove(w)
        }
    }

    fn timer_page(&self, unit_creation_type: &UnitCreateType) {
        if let Some(widget) = self.section.borrow().get(&UnitCreateType::Timer) {
            if widget.parent().is_none() {
                self.carousel.append(widget);
            }
            widget.set_property(PROPERTY_NAME, unit_creation_type);
        } else {
            let timer_page = TimerCreatorPage::new(self.obj().downgrade());
            timer_page.set_property(PROPERTY_NAME, unit_creation_type);
            self.add_page(&UnitCreateType::Timer, timer_page);
        }
    }

    fn service_page(&self, unit_type: &UnitCreateType) {
        if let Some(widget) = self.section.borrow().get(&UnitCreateType::Service) {
            if widget.parent().is_none() {
                self.carousel.append(widget);
            }
            widget.set_property(PROPERTY_NAME, unit_type);
        } else {
            let service_page = ServiceCreatorPage::default();
            service_page.set_property(PROPERTY_NAME, unit_type);
            self.add_page(&UnitCreateType::Service, service_page);
        }
    }

    fn add_page<T: IsA<gtk::Widget>>(&self, unit_type: &UnitCreateType, widget: T) {
        self.carousel.append(&widget);
        self.section.borrow_mut().insert(*unit_type, widget.into());
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

        match systemd::list_unit_files(level).await {
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
    }

    pub fn get_trigger_units(&self) -> Ref<'_, HashSet<String>> {
        let level = self.level.get();

        match level {
            UnitDBusLevel::System | UnitDBusLevel::Both => self.system_file_list.borrow(),
            UnitDBusLevel::UserSession => self.session_file_list.borrow(),
        }
    }

    fn is_fill_exist(&self, unit_prefix: &str) -> bool {
        if let Some(state) = self
            .action_group
            .borrow()
            .action_state(&ACTION_CREATOR_UNIT_BUS[8..])
        {
            let level: UnitDBusLevel = (&state).into();
            let set = match level {
                UnitDBusLevel::System | UnitDBusLevel::Both => self.system_file_list.borrow(),
                UnitDBusLevel::UserSession => self.session_file_list.borrow(),
            };

            match self.creation_type.get() {
                UnitCreateType::Service => set.contains(&format!("{unit_prefix}.service")),
                UnitCreateType::Timer => set.contains(&format!("{unit_prefix}.timer")),
                UnitCreateType::TimerService => {
                    set.contains(&format!("{unit_prefix}.service"))
                        || set.contains(&format!("{unit_prefix}.timer"))
                }
                UnitCreateType::Unknown => false,
            }
        } else {
            false
        }
    }

    fn validate_entry(&self) {
        let entry = self.unit_name_prefix.get();
        let text = entry.text();

        let text = text.as_str();

        let name_err = if text.is_empty() {
            UnitNameErr::Empty
        } else {
            if self.creation_type.get().max_sufix_len() + text.len() > 255 {
                UnitNameErr::Limit255
            } else if !self
                .re
                .get_or_init(|| regex::Regex::new(VALID_UNIT_NAME).unwrap())
                .is_match(text)
            {
                UnitNameErr::WrongChar
            } else if self.is_fill_exist(text) {
                UnitNameErr::FileExits
            } else {
                UnitNameErr::NoErr
            }
        };

        match name_err {
            UnitNameErr::NoErr => {
                entry.remove_css_class("warning");
            }
            _ => {
                entry.add_css_class("warning");
            }
        }
        entry.set_title(&name_err.title_err());

        println!("{}", text);
    }
}

enum UnitNameErr {
    WrongChar,
    Limit255,
    FileExits,
    Empty,
    NoErr,
}

impl UnitNameErr {
    fn title_err(&self) -> String {
        let pre = "Unit Name Prefix";
        match self {
            UnitNameErr::WrongChar => format!("{pre} - Wrong Char"),
            UnitNameErr::Limit255 => format!("{pre} - Unit File over 255 characters"),
            UnitNameErr::FileExits => format!("{pre} - Unit File already exists"),
            UnitNameErr::Empty => format!("{pre} - Name Empty"),
            UnitNameErr::NoErr => pre.to_owned(),
        }
    }
}

impl ObjectImpl for UnitCreatorWindowImp {
    fn constructed(&self) {
        self.parent_constructed();

        let event_controller = clear_on_escape();
        self.unit_name_prefix.add_controller(event_controller);

        self.insert_page(&UnitCreateType::Service);
        {
            let creator_window = self.obj().clone().downgrade();
            self.unit_name_prefix.connect_changed(move |_| {
                upgrade!(creator_window).imp().validate_entry();
            });
        }

        const ACTION_CREATOR_UNIT_TYPE_SELECTION: &str = "creator.unit_type_selection";
        let preferences_action_entry: gio::ActionEntry<_> = {
            let unit_creator_window = self.obj().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_TYPE_SELECTION[8..])
                .activate(move |_, action, param| {
                    debug!("{} {:?}", ACTION_CREATOR_UNIT_TYPE_SELECTION, param);
                    if let Some(param) = param {
                        action.set_state(param);

                        let creation_window = upgrade!(unit_creator_window);
                        let creation_window = creation_window.imp();
                        creation_window.set_unit_type(param);
                        creation_window.validate_entry();
                    }
                })
                .parameter_type(Some(glib::VariantTy::STRING))
                .state("service".to_variant())
                .build()
        };

        let action_creator_bus: gio::ActionEntry<_> = {
            let creation_window = self.obj().clone().downgrade();
            gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_BUS[8..])
                .activate(move |_, action, param| {
                    debug!("{} {:?}", ACTION_CREATOR_UNIT_BUS, param);
                    if let Some(param) = param {
                        action.set_state(param);
                        let creation_window = creation_window.clone();
                        let param = param.clone();

                        let creation_window = upgrade!(creation_window);
                        let level: UnitDBusLevel = param.into();
                        creation_window.imp().level.set(level);
                        glib::spawn_future_local(async move {
                            let creation_window = creation_window.imp();
                            creation_window.fill_unit_files().await;
                            creation_window.validate_entry();
                        });
                    }
                })
                .parameter_type(Some(glib::VariantTy::STRING))
                .state("system".to_variant())
                .build()
        };

        let action_group = self.action_group.borrow().clone();
        action_group.add_action_entries([preferences_action_entry, action_creator_bus]);
        self.obj()
            .insert_action_group("creator", Some(&action_group));

        let creation_window = self.obj().clone();

        if let Some(state) = action_group.action_state(&ACTION_CREATOR_UNIT_BUS[8..]) {
            self.level.set(state.into());
        }

        glib::spawn_future_local(async move {
            creation_window.imp().fill_unit_files().await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_name_regex() {
        let re = regex::Regex::new(VALID_UNIT_NAME).unwrap();

        // Valid cases: alphanumeric, underscore, hyphen
        assert!(re.is_match("service1"));
        assert!(re.is_match("my-service"));
        assert!(re.is_match("unit_name"));
        assert!(re.is_match("unit_name@"));
        assert!(re.is_match("Unit123"));
        assert!(re.is_match("a"));
        assert!(re.is_match("1"));
        assert!(re.is_match("_"));
        assert!(re.is_match("-"));
        assert!(re.is_match("org.freedesktop.network1"));
        assert!(re.is_match(r"org\freedesktop\network1"));
        assert!(re.is_match(r"org:freedesktop:network1"));

        // Invalid cases: spaces, special characters, empty string
        assert!(!re.is_match("service with space"));
        assert!(!re.is_match("service@domain"));
        assert!(!re.is_match(""));
        assert!(!re.is_match("service/"));
        assert!(!re.is_match("service name"));
        assert!(!re.is_match("service\tname"));
    }
}
