use crate::widget::creator::{PageType, UnitCreatorWindow};
use adw::prelude::NavigationPageExt;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::{
    glib::{self},
    prelude::EditableExt,
};

glib::wrapper! {
    pub struct UnitCreatorFirstPage(ObjectSubclass<imp::UnitCreatorFirstPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl UnitCreatorFirstPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>, page: PageType) -> Self {
        let obj: UnitCreatorFirstPage = glib::Object::new();
        obj.set_tag(Some(page.id()));
        obj.imp().set_window(window);
        obj
    }

    pub fn fetch_settings(&self) -> (bool, glib::GString) {
        let runtime = self.imp().runtime_switch.state();
        let prefix = self.imp().unit_name_prefix.text();
        (runtime, prefix)
    }
}

mod imp {

    use std::cell::OnceCell;

    use crate::{
        upgrade, upgrade_opt,
        widget::{
            self,
            creator::{
                ACTION_CREATOR_UNIT_BUS, ACTION_CREATOR_UNIT_TYPE_SELECTION, CreateUnitErr,
                UnitCreateType, UnitCreatorWindow, VALID_UNIT_NAME,
            },
        },
    };

    use super::*;
    use adw::{prelude::PreferencesRowExt, subclass::prelude::*};
    use base::enums::UnitDBusLevel;
    use gettextrs::pgettext;
    use glib::WeakRef;
    use gtk::{glib, prelude::*};
    use regex::Regex;
    use tracing::{debug, error, warn};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/create_first.ui")]
    #[properties(wrapper_type = super::UnitCreatorFirstPage)]
    pub struct UnitCreatorFirstPageImp {
        #[template_child]
        pub(super) unit_name_prefix: TemplateChild<adw::EntryRow>,
        #[template_child]
        radio_button_service: TemplateChild<adw::ActionRow>,
        #[template_child]
        radio_button_timer_service: TemplateChild<adw::ActionRow>,
        #[template_child]
        radio_button_timer: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) runtime_switch: TemplateChild<gtk::Switch>,

        re: OnceCell<Regex>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,
    }

    impl UnitCreatorFirstPageImp {
        fn validate_entry(&self) {
            let entry = self.unit_name_prefix.get();
            let text = entry.text();

            let text = text.as_str();

            let name_err = if text.is_empty() {
                CreateUnitErr::Empty
            } else {
                let window = upgrade_opt!(self.window.get());
                if window.creation_type().max_sufix_len() + text.len() > 255 {
                    CreateUnitErr::Limit255
                } else if !self
                    .re
                    .get_or_init(|| regex::Regex::new(VALID_UNIT_NAME).unwrap())
                    .is_match(text)
                {
                    CreateUnitErr::WrongChar
                } else if self.is_fill_exist(text) {
                    CreateUnitErr::FileExits
                } else {
                    CreateUnitErr::NoErr
                }
            };

            match name_err {
                CreateUnitErr::NoErr => {
                    entry.remove_css_class("warning");
                }
                _ => {
                    entry.add_css_class("warning");
                }
            }

            let prefix = pgettext("creator", "Unit Name Prefix");
            entry.set_title(&name_err.title_err(&prefix));
        }

        pub(super) fn set_window(&self, window: WeakRef<UnitCreatorWindow>) {
            let _ = self.window.set(window.clone());
            let event_controller = widget::clear_on_escape();
            self.unit_name_prefix.add_controller(event_controller);

            let window = upgrade!(window);
            window.set_creation_unit_type(UnitCreateType::Service);
            // window.insert_page(&UnitCreateType::Service);
            {
                let creator_window = self.obj().downgrade();
                self.unit_name_prefix.connect_changed(move |_| {
                    upgrade!(creator_window).imp().validate_entry();
                });
            }

            let preferences_action_entry: gio::ActionEntry<_> = {
                let first_page = self.obj().downgrade();
                let creation_window = window.downgrade();
                gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_TYPE_SELECTION[8..])
                    .activate(move |_, action, param| {
                        debug!("{} {:?}", ACTION_CREATOR_UNIT_TYPE_SELECTION, param);
                        if let Some(param) = param {
                            action.set_state(param);

                            // let creation_window = upgrade!(unit_creator_window);
                            let creation_window = upgrade!(creation_window);
                            let first_page = upgrade!(first_page);
                            // let creation_window = creation_window.imp();
                            let unit_creation_type: UnitCreateType = param.into();
                            creation_window.set_creation_type(unit_creation_type);
                            first_page.imp().validate_entry();
                        }
                    })
                    .parameter_type(Some(glib::VariantTy::STRING))
                    .state(UnitCreateType::Timer.id().to_variant())
                    .build()
            };

            let action_creator_bus: gio::ActionEntry<_> = {
                let creation_window = window.downgrade();
                let first_page = self.obj().downgrade();

                gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_BUS[8..])
                    .activate(move |_, action, param| {
                        debug!("{} {:?}", ACTION_CREATOR_UNIT_BUS, param);
                        if let Some(param) = param {
                            action.set_state(param);
                            let creation_window = creation_window.clone();
                            let param = param.clone();

                            let creation_window = upgrade!(creation_window);
                            let first_page = upgrade!(first_page);
                            let level: UnitDBusLevel = param.into();
                            creation_window.set_bus_level(level);
                            glib::spawn_future_local(async move {
                                let creation_window = creation_window.imp();
                                creation_window.fill_unit_files().await;
                                first_page.imp().validate_entry();
                            });
                        }
                    })
                    .parameter_type(Some(glib::VariantTy::STRING))
                    .state("system".to_variant())
                    .build()
            };

            let donate: gio::ActionEntry<_> = gio::ActionEntry::builder("donate")
                .activate(|_, _, _| {
                    let launcher = gtk::UriLauncher::new("https://github.com/sponsors/plrigaux");
                    launcher.launch(
                        None::<&gtk::Window>,
                        None::<&gio::Cancellable>,
                        move |result| {
                            if let Err(error) = result {
                                warn!("Finished launch $upport Error {error:?}")
                            }
                        },
                    );
                })
                .build();

            let action_group = window.imp().action_group.borrow().clone();
            action_group.add_action_entries([preferences_action_entry, action_creator_bus, donate]);
            self.obj()
                .insert_action_group("creator", Some(&action_group));

            if let Some(state) = action_group.action_state(&ACTION_CREATOR_UNIT_BUS[8..]) {
                let level: UnitDBusLevel = state.into();
                window.set_bus_level(level);
            }

            let window = window.clone();
            glib::spawn_future_local(async move {
                window.imp().fill_unit_files().await;
            });

            action_group.activate_action(
                &ACTION_CREATOR_UNIT_TYPE_SELECTION[8..],
                Some(&UnitCreateType::Timer.id().to_variant()),
            );
        }

        fn is_fill_exist(&self, unit_prefix: &str) -> bool {
            let Some(window) = self.window.get().and_then(|w| w.upgrade()) else {
                error!("No parent window");
                return false;
            };

            if let Some(state) = window
                .action_group()
                .action_state(&ACTION_CREATOR_UNIT_BUS[8..])
            {
                let level: UnitDBusLevel = (&state).into();
                let set = match level {
                    UnitDBusLevel::System | UnitDBusLevel::Both => window.system_file_list(),
                    UnitDBusLevel::UserSession => window.session_file_list(),
                };

                match window.creation_type() {
                    UnitCreateType::Service => set.contains(&format!("{unit_prefix}.service")),
                    UnitCreateType::Timer => set.contains(&format!("{unit_prefix}.timer")),
                    UnitCreateType::TimerService => {
                        set.contains(&format!("{unit_prefix}.service"))
                            || set.contains(&format!("{unit_prefix}.timer"))
                    }
                }
            } else {
                false
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitCreatorFirstPageImp {
        const NAME: &'static str = "UnitFileCreatorFirstPage";
        type Type = UnitCreatorFirstPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            // klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for UnitCreatorFirstPageImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl UnitCreatorFirstPageImp {}

    impl WidgetImpl for UnitCreatorFirstPageImp {}

    impl NavigationPageImpl for UnitCreatorFirstPageImp {}
}
#[cfg(test)]
mod tests {
    use crate::widget::creator::VALID_UNIT_NAME;

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
