use glib::subclass::types::ObjectSubclassIsExt;
use gtk::glib::{self};

use crate::widget::creator::{PageType, UnitCreateType};

glib::wrapper! {
    pub struct NavigationRow(ObjectSubclass<imp::NavigationRowImp>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget, gtk::Orientable;
}

impl NavigationRow {
    pub fn new() -> Self {
        let obj: NavigationRow = glib::Object::new();
        obj
    }

    pub(crate) fn set_page_type(&self, page: PageType, creation_type: UnitCreateType) {
        self.imp().set_page_type(page, creation_type);
    }
}

mod imp {

    use super::*;
    use adw::subclass::prelude::*;
    use gtk::{glib, prelude::WidgetExt};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/nav_row.ui")]
    #[properties(wrapper_type = super::NavigationRow)]
    pub struct NavigationRowImp {
        #[template_child]
        prev_button: TemplateChild<gtk::Button>,
        #[template_child]
        file_button: TemplateChild<gtk::Button>,
        #[template_child]
        next_button: TemplateChild<gtk::Button>,
        #[template_child]
        create_button: TemplateChild<gtk::Button>,
    }

    impl NavigationRowImp {
        pub(super) fn set_page_type(&self, page: PageType, creation_type: UnitCreateType) {
            match (page, creation_type) {
                (PageType::Start, _) => {
                    self.prev_button.set_visible(false);
                    self.file_button.set_visible(false);
                    self.next_button.set_visible(true);
                    self.create_button.set_visible(false);
                }
                (PageType::Service, _) => {
                    self.prev_button.set_visible(true);
                    self.file_button.set_visible(true);
                    self.next_button.set_visible(true);
                    self.create_button.set_visible(false);
                }
                (PageType::Timer, _) => {
                    self.prev_button.set_visible(true);
                    self.file_button.set_visible(true);
                    self.next_button.set_visible(true);
                    self.create_button.set_visible(false);
                }
                (PageType::Launch, _) => {
                    self.prev_button.set_visible(true);
                    self.file_button.set_visible(false);
                    self.next_button.set_visible(false);
                    self.create_button.set_visible(true);
                }
                (PageType::ServiceFile, _) => {
                    self.prev_button.set_visible(true);
                    self.file_button.set_visible(false);
                    self.next_button.set_visible(true);
                    self.create_button.set_visible(false);
                }
                (PageType::TimerFile, _) => {
                    self.prev_button.set_visible(true);
                    self.file_button.set_visible(false);
                    self.next_button.set_visible(true);
                    self.create_button.set_visible(false);
                }
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NavigationRowImp {
        const NAME: &'static str = "NavigationRow";
        type Type = NavigationRow;
        type ParentType = gtk::Box;

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
    impl ObjectImpl for NavigationRowImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl NavigationRowImp {}

    impl WidgetImpl for NavigationRowImp {}

    impl BoxImpl for NavigationRowImp {}
}
