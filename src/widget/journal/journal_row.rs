use gtk::{glib, pango::AttrList, subclass::prelude::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct JournalRow(ObjectSubclass<imp::JournalRowImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl JournalRow {
    pub fn new() -> Self {
        let obj: JournalRow = glib::Object::new();
        obj
    }

    pub fn clear(&self) {
        self.imp().clear();
    }

    pub fn set_text(&self, prefix: &str, message: &str) {
        self.imp().set_text(prefix, message);
    }

    pub fn set_message_attributes(&self, attributes : Option<&AttrList>){
        self.imp().set_message_attributes(attributes);
    }
}

mod imp {
    use gtk::{glib, pango::AttrList, subclass::prelude::*};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/journal_row.ui")]
    pub struct JournalRowImp {
        #[template_child]
        prefix_label: gtk::TemplateChild<gtk::Label>,

        #[template_child]
        message_label: gtk::TemplateChild<gtk::Label>,
    }

    impl JournalRowImp {
        pub(super) fn clear(&self) {
            self.prefix_label.set_text("");
            self.message_label.set_text("");

            //self.prefix_label.set_attributes(None);
            self.message_label.set_attributes(None);
        }

        pub(super) fn set_text(&self, prefix: &str, message: &str) {
            self.prefix_label.set_text(prefix);
            self.message_label.set_text(message);
        }

        pub(super) fn set_message_attributes(&self, attributes : Option<&AttrList>){
            self.message_label.set_attributes(attributes);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for JournalRowImp {
        const NAME: &'static str = "JournalRow";
        type Type = super::JournalRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for JournalRowImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for JournalRowImp {}
    impl BoxImpl for JournalRowImp {}
}
