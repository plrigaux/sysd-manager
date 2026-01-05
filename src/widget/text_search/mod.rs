use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{glib, prelude::WidgetExt};

glib::wrapper! {
    pub struct TextSearchBar(ObjectSubclass<imp::TextSearchBarImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl TextSearchBar {
    pub fn new() -> TextSearchBar {
        let obj: TextSearchBar = glib::Object::new();
        obj
    }

    pub fn grab_focus_on_search_entry(&self) {
        self.imp().search_entry.grab_focus();
    }
}

mod imp {
    use std::cell::OnceCell;

    use gtk::{glib, subclass::prelude::*};

    use crate::widget::unit_list::UnitListPanel;

    use super::TextSearchBar;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/text_search.ui")]
    pub struct TextSearchBarImp {
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,

        #[template_child]
        case_sensitive_toggle_button: TemplateChild<gtk::ToggleButton>,

        #[template_child]
        regex_toggle_button: TemplateChild<gtk::ToggleButton>,

        #[template_child]
        previous_button: TemplateChild<gtk::Button>,

        #[template_child]
        next_button: TemplateChild<gtk::Button>,
    }

    #[gtk::template_callbacks]
    impl TextSearchBarImp {
        pub(crate) fn clear(&self) {}
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TextSearchBarImp {
        const NAME: &'static str = "TextSearch";
        type Type = TextSearchBar;
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

    impl ObjectImpl for TextSearchBarImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for TextSearchBarImp {}
    impl BoxImpl for TextSearchBarImp {}
}
