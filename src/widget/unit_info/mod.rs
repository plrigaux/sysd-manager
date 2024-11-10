use crate::systemd::data::UnitInfo;

mod construct_info;
mod time_handling;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct UnitInfoPanel(ObjectSubclass<imp::UnitInfoPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitInfoPanel {
    pub fn new(is_dark: bool) -> Self {
        // Create new window
        let obj: UnitInfoPanel = glib::Object::new();

        obj.set_dark(is_dark);

        obj
    }

    pub fn display_unit_info(&self, unit: &UnitInfo) {
        self.imp().display_unit_info(unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{
        glib,
        prelude::*,
        subclass::{
            box_::BoxImpl,
            prelude::*,
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        TemplateChild,
    };

    use log::{info, warn};

    use crate::{systemd::data::UnitInfo, widget::info_window::InfoWindow};

    use super::construct_info::fill_all_info;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
    pub struct UnitInfoPanelImp {
        #[template_child]
        show_all_button: TemplateChild<gtk::Button>,

        #[template_child]
        refresh_button: TemplateChild<gtk::Button>,

        #[template_child]
        unit_info_textview: TemplateChild<gtk::TextView>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl UnitInfoPanelImp {
        #[template_callback]
        fn refresh_info_clicked(&self, button: &gtk::Button) {
            info!("button {:?}", button);

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            self.update_unit_info(&unit)
        }

        #[template_callback]
        fn show_all_clicked(&self, _button: &gtk::Button) {
            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            let info_window = InfoWindow::new();

            info!("show_all_clicked {:?}", unit.primary());

            info_window.fill_data(&unit);

            info_window.present();
        }

        pub(crate) fn display_unit_info(&self, unit: &UnitInfo) {
            let _old = self.unit.replace(Some(unit.clone()));

            self.update_unit_info(&unit)
        }

        /// Updates the associated journal `TextView` with the contents of the unit's journal log.
        fn update_unit_info(&self, unit: &UnitInfo) {
            let text = fill_all_info(unit, self.is_dark.get());

            let unit_info_text_view: &gtk::TextView = self.unit_info_textview.as_ref();

            let buf = unit_info_text_view.buffer();

            buf.set_text(""); // clear text

            let mut start_iter = buf.start_iter();

            buf.insert_markup(&mut start_iter, &text);
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitInfoPanelImp {
        const NAME: &'static str = "UnitInfoPanel";
        type Type = super::UnitInfoPanel;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitInfoPanelImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for UnitInfoPanelImp {}
    impl BoxImpl for UnitInfoPanelImp {}
}
