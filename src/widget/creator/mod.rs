mod service_creator_page;
mod timer_creator_page;
use crate::widget::app_window::AppWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

glib::wrapper! {

    pub struct UnitCreatorWindow(ObjectSubclass<imp::UnitCreatorWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitCreatorWindow {
    pub fn new(app_window: &AppWindow) -> Self {
        let obj: UnitCreatorWindow = glib::Object::new();
        let _ = obj.imp().app_window.set(app_window.clone());
        obj
    }
}

mod imp {

    const WINDOW_HEIGHT: &str = "unit-creator-window-height";
    const WINDOW_WIDTH: &str = "unit-creator-window-width";
    use super::UnitCreatorWindow;
    use crate::{
        upgrade,
        widget::{
            app_window::AppWindow,
            creator::{
                service_creator_page::ServiceCreatorPage, timer_creator_page::TimerCreatorPage,
            },
        },
    };
    use adw::subclass::window::AdwWindowImpl;
    use gio::{SimpleActionGroup, prelude::ActionMapExtManual};
    use glib::{object::Cast, property::PropertyGet, variant::ToVariant};
    use gtk::{
        glib::{self},
        prelude::{GtkWindowExt, WidgetExt},
        subclass::{
            prelude::*,
            widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
        },
    };
    use std::{
        cell::{OnceCell, RefCell},
        collections::HashMap,
    };
    use tracing::{debug, error, info, warn};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/creator.ui")]
    pub struct UnitCreatorWindowImp {
        #[template_child]
        carousel: TemplateChild<adw::Carousel>,

        section: RefCell<HashMap<UnitCreateType, gtk::Widget>>,

        pub(super) app_window: OnceCell<AppWindow>,
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

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    enum UnitCreateType {
        Service,
        Timer,
        TimerService,
        Unknown,
    }

    impl UnitCreateType {}

    impl From<&glib::Variant> for UnitCreateType {
        fn from(value: &glib::Variant) -> Self {
            match value.get::<String>().as_deref() {
                Some("service") => UnitCreateType::Service,
                Some("timer") => UnitCreateType::Timer,
                Some("timer_service") => UnitCreateType::TimerService,
                other => {
                    warn!("Unkown type {:?}", other);
                    UnitCreateType::Unknown
                }
            }
        }
    }

    impl UnitCreatorWindowImp {
        fn set_unit_type(&self, unit_type: &glib::Variant) {
            let unit_type: UnitCreateType = unit_type.into();
            match unit_type {
                UnitCreateType::Service => {
                    self.insert_page(&unit_type);
                }
                UnitCreateType::Timer => {
                    self.insert_page(&unit_type);
                }
                UnitCreateType::TimerService => {}
                UnitCreateType::Unknown => {}
            }
        }

        fn insert_page(&self, unit_type: &UnitCreateType) {
            for (_t, w) in self
                .section
                .borrow()
                .iter()
                .filter(|(create_type, _)| *create_type != unit_type)
            {
                self.carousel.remove(w);
            }

            if let Some(widget) = self.section.borrow().get(unit_type) {
                self.carousel.append(widget);
            } else {
                let widget = match unit_type {
                    UnitCreateType::Service => {
                        let service = ServiceCreatorPage::default();

                        Some(service.upcast::<gtk::Widget>())
                    }
                    UnitCreateType::Timer => {
                        let timer_page = TimerCreatorPage::default();

                        Some(timer_page.upcast::<gtk::Widget>())
                    }
                    _ => None,
                };

                if let Some(widget) = widget {
                    self.carousel.append(&widget);
                    self.section.borrow_mut().insert(*unit_type, widget);
                }
            };
        }
    }

    impl ObjectImpl for UnitCreatorWindowImp {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().child();

            self.insert_page(&UnitCreateType::Service);

            const ACTION_CREATOR_UNIT_TYPE_SELECTION: &str = "creator.unit_type_selection";
            let preferences_action_entry: gio::ActionEntry<_> = {
                let unit_creator_window = self.obj().downgrade();
                gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_TYPE_SELECTION[8..])
                    .activate(move |_, action, param| {
                        debug!("{} {:?}", ACTION_CREATOR_UNIT_TYPE_SELECTION, param);
                        if let Some(param) = param {
                            action.set_state(param);

                            let unit_creator_window = upgrade!(unit_creator_window);

                            unit_creator_window.imp().set_unit_type(param);
                        }
                    })
                    .parameter_type(Some(glib::VariantTy::STRING))
                    .state("service".to_variant())
                    .build()
            };
            const ACTION_CREATOR_UNIT_BUS: &str = "creator.unit_bus_selection";

            let action_creator_bus: gio::ActionEntry<_> =
                gio::ActionEntry::builder(&ACTION_CREATOR_UNIT_BUS[8..])
                    .activate(|_, action, param| {
                        debug!("{} {:?}", ACTION_CREATOR_UNIT_BUS, param);
                        if let Some(param) = param {
                            action.set_state(param);
                        }
                        // if let Some(win) = application.active_window() {
                        //     let app_window: Option<&AppWindow> = win.downcast_ref::<AppWindow>();

                        //     let pdialog = PreferencesDialog::new(app_window);
                        //     pdialog.present(Some(&win));
                        //     //pdialog.present(Some(&win));
                        //     //gtk::prelude::GtkWindowExt::present(&pdialog);
                        // } else {
                        //     let pdialog = PreferencesDialog::new(None);
                        //     pdialog.present(None::<&gtk::Widget>);
                        // }
                    })
                    .parameter_type(Some(glib::VariantTy::STRING))
                    .state("system".to_variant())
                    .build();
            let action_group = SimpleActionGroup::new();

            action_group.add_action_entries([preferences_action_entry, action_creator_bus]);
            self.obj()
                .insert_action_group("creator", Some(&action_group));
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
}
