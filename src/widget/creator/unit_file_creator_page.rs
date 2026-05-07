use glib::{GString, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

use crate::widget::creator::unit_file::UnitFileData;

glib::wrapper! {

    pub struct UnitFileCreatorPage(ObjectSubclass<imp::UnitFileCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl UnitFileCreatorPage {
    pub fn new() -> Self {
        let obj: UnitFileCreatorPage = glib::Object::new();
        obj
    }

    pub fn update_view(&self, data: &UnitFileData) {
        self.imp().update_view(data)
    }

    pub fn file_text(&self) -> GString {
        self.imp().file_text()
    }
}

mod imp {

    use std::cell::OnceCell;

    use super::*;
    use crate::widget::preferences::{data::PREFERENCES, style_scheme::set_new_style_scheme};
    use adw::subclass::prelude::*;
    use glib::GString;
    use gtk::{glib, prelude::*};
    use sourceview5::prelude::*;
    use tracing::warn;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_creator_page.ui")]
    #[properties(wrapper_type = super::UnitFileCreatorPage)]
    pub struct UnitFileCreatorPageImp {
        #[template_child]
        unit_file_scrolled_window: TemplateChild<gtk::ScrolledWindow>,

        pub(super) buffer: OnceCell<sourceview5::Buffer>,
    }

    impl UnitFileCreatorPageImp {}

    #[glib::object_subclass]
    impl ObjectSubclass for UnitFileCreatorPageImp {
        const NAME: &'static str = "UnitFileCreatorPage";
        type Type = UnitFileCreatorPage;
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
    impl ObjectImpl for UnitFileCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();

            let buffer = sourceview5::Buffer::new(None);

            if let Some(ref language) = sourceview5::LanguageManager::new().language("ini") {
                buffer.set_language(Some(language));
            }

            let _ = self.buffer.set(buffer.clone());
            let style_scheme_id = PREFERENCES.unit_file_style_scheme();
            set_new_style_scheme(&buffer, Some(&style_scheme_id));
            let view = sourceview5::View::with_buffer(&buffer);
            view.set_show_line_numbers(true);
            view.set_highlight_current_line(true);
            view.set_tab_width(4);
            view.set_monospace(true);
            view.set_wrap_mode(gtk::WrapMode::WordChar);

            self.unit_file_scrolled_window.set_child(Some(&view));
        }
    }

    impl UnitFileCreatorPageImp {
        pub(super) fn update_view(&self, data: &UnitFileData) {
            let Some(buffer) = self.buffer.get() else {
                warn!("No buffer");
                return;
            };

            let data_string = data.to_file_data();
            buffer.set_text(&data_string);
        }

        pub(crate) fn file_text(&self) -> GString {
            let Some(buffer) = self.buffer.get() else {
                warn!("No buffer");
                return GString::new();
            };

            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.text(&start, &end, true)
        }
    }

    impl WidgetImpl for UnitFileCreatorPageImp {}

    impl NavigationPageImpl for UnitFileCreatorPageImp {}
}
