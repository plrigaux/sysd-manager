pub mod dosini;

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

        let system_manager = adw::StyleManager::default();

        let is_dark = system_manager.is_dark();

        obj.set_dark(is_dark);

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

    use crate::{
        systemd::{self, data::UnitInfo},
        widget::preferences::data::PREFERENCES,
    };

    use super::dosini;

    const SUGGESTED_ACTION: &str = "suggested-action";

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
    pub struct UnitFilePanelImp {
        #[template_child]
        save_button: TemplateChild<gtk::Button>,

        #[template_child]
        file_path_label: TemplateChild<gtk::Label>,

        #[template_child]
        unit_file_text: TemplateChild<gtk::TextView>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
    }

    #[gtk::template_callbacks]
    impl UnitFilePanelImp {
        #[template_callback]
        fn save_file(&self, button: &gtk::Button) {
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

            let _old = self.unit.replace(Some(unit.clone()));

            self.set_text(file_content);
        }

        fn set_text(&self, file_content: &str) {
            let in_color = PREFERENCES.unit_file_colors();

            let buf = self.unit_file_text.buffer();
            if in_color {
                buf.set_text("");

                let is_dark = self.is_dark.get();
                let mut start_iter = buf.start_iter();

                let text = dosini::convert_to_mackup(&file_content, is_dark);
                buf.insert_markup(&mut start_iter, &text);
            } else {
                buf.set_text(&file_content);
            }

            self.save_button.remove_css_class(SUGGESTED_ACTION);
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);

            //get current text
            let buffer = self.unit_file_text.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let file_content = buffer.text(&start, &end, true);

            self.set_text(file_content.as_str());
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

    impl ObjectImpl for UnitFilePanelImp {
        fn constructed(&self) {
            self.parent_constructed();
            {
                let buffer = self.unit_file_text.buffer();
                let save_button = self.save_button.clone();
                buffer.connect_begin_user_action(move |_buf| {
                    save_button.add_css_class(SUGGESTED_ACTION);
                });
            }
        }
    }
    impl WidgetImpl for UnitFilePanelImp {}
    impl BoxImpl for UnitFilePanelImp {}
}
