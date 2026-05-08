use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

use crate::widget::creator::UnitCreatorWindow;

glib::wrapper! {

    pub struct LaunchCreatorPage(ObjectSubclass<imp::LaunchCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl LaunchCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>) -> Self {
        let obj: LaunchCreatorPage = glib::Object::new();
        obj.imp().set_window(window);
        obj
    }
}

mod imp {

    use super::*;
    use crate::{upgrade, widget::creator::UnitCreateType};
    use adw::subclass::prelude::*;
    use gtk::{glib, prelude::*};
    use std::cell::{Cell, OnceCell};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/launch_creator_page.ui")]
    #[properties(wrapper_type = super::LaunchCreatorPage)]
    pub struct LaunchCreatorPageImp {
        #[property(get, set, default)]
        creation_type: Cell<UnitCreateType>,

        #[template_child]
        daemon_reload_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        enable_switch: TemplateChild<adw::SwitchRow>,
        #[template_child]
        start_switch: TemplateChild<adw::SwitchRow>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,
        // #[property(get)]
        // pub(super) data: OnceCell<UnitFileData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LaunchCreatorPageImp {
        const NAME: &'static str = "LaunchCreatorPage";
        type Type = LaunchCreatorPage;
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
    impl ObjectImpl for LaunchCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();

            let daemon_reload_active = self.daemon_reload_switch.is_active();
            self.enable_switch.set_sensitive(daemon_reload_active);
            self.start_switch.set_sensitive(daemon_reload_active);
        }
    }

    impl LaunchCreatorPageImp {
        pub fn set_window(&self, window_weak: WeakRef<UnitCreatorWindow>) {
            let _ = self.window.set(window_weak.clone());

            let window = upgrade!(window_weak);

            const ACTION_CREATOR_DAEMON_RELOAD: &str = "creator.daemon-reload";
            let daemon_reload_entry: gio::ActionEntry<_> = {
                let enable_switch = self.enable_switch.downgrade();
                let start_switch = self.start_switch.downgrade();
                gio::ActionEntry::builder(&ACTION_CREATOR_DAEMON_RELOAD[8..])
                    .activate(move |_, action, _| {
                        let Some(state) = action.state().and_then(|var| var.get::<bool>()) else {
                            return;
                        };
                        let state = !state;
                        action.set_state(&(state).to_variant());

                        if let Some(enable_switch) = enable_switch.upgrade() {
                            enable_switch.set_sensitive(state);
                            enable_switch.set_active(false);
                        }

                        if let Some(start_switch) = start_switch.upgrade() {
                            start_switch.set_sensitive(state);
                            start_switch.set_active(false);
                        }
                    })
                    .parameter_type(Some(glib::VariantTy::BOOLEAN))
                    .state(false.to_variant())
                    .build()
            };

            let action_group = window.action_group();
            action_group.add_action_entries([daemon_reload_entry]);
        }
    }

    impl WidgetImpl for LaunchCreatorPageImp {}

    impl NavigationPageImpl for LaunchCreatorPageImp {}
}
