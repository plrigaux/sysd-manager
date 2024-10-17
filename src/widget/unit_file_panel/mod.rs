use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};

use crate::systemd::{self, data::UnitInfo};

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitFilePanel(ObjectSubclass<imp::UnitFilePanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitFilePanel {
    pub fn new() -> Self {
        // Create new window
        let obj: UnitFilePanel = glib::Object::new();

        obj
    }

    pub fn set_file_content(&self, unit: &UnitInfo) {
        let file_content = systemd::get_unit_file_info(&unit);

        let file_path = match unit.file_path() {
            Some(s) => s,
            None => "".to_owned(),
        };

        self.imp()
            .display_file_info(&file_path, &file_content, unit);
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{
        glib,
        prelude::*,
        subclass::{
            box_::BoxImpl,
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{
                CompositeTemplateCallbacksClass, CompositeTemplateClass,
                CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
            },
        },
        TemplateChild,
    };

    use log::{info, warn};

    use crate::{
        systemd::{self, data::UnitInfo},
        widget::button_icon::ButtonIcon,
    };

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
    pub struct UnitFilePanelImp {
        #[template_child]
        save_button: TemplateChild<ButtonIcon>,

        #[template_child]
        file_path_label: TemplateChild<gtk::Label>,

        #[template_child]
        unit_file_text: TemplateChild<gtk::TextView>,

        pub unit: RefCell<Option<UnitInfo>>,
    }

    #[gtk::template_callbacks]
    impl UnitFilePanelImp {
        
        #[template_callback]
        fn save_file(&self, button: &ButtonIcon) {
            info!("button {:?}", button);

            let binding = self.unit.borrow();
            let Some(unit) = binding.as_ref() else {
                warn!("no unit file");
                return;
            };

            let buffer = self.unit_file_text.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true);

            systemd::save_text_to_file(unit, &text);
        }

        pub(crate) fn display_file_info(
            &self,
            file_path: &str,
            file_content: &str,
            unit: &UnitInfo,
        ) {
            self.file_path_label.set_label(file_path);

            self.unit_file_text.buffer().set_text(file_content);

            let _old = self.unit.replace(Some(unit.clone()));
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitFilePanelImp {
        const NAME: &'static str = "UnitFilePanel";
        type Type = super::UnitFilePanel;
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

    impl ObjectImpl for UnitFilePanelImp {}
    impl WidgetImpl for UnitFilePanelImp {}
    impl BoxImpl for UnitFilePanelImp {}
}
