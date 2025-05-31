use glib::Object;
use gtk::glib;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct SignalRow(ObjectSubclass<imp::SignalRowImp>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for SignalRow {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_type_text(&self, text: &str) {
        self.imp().signal_type.set_label(text);
    }

    pub fn set_details_text(&self, text: &str) {
        self.imp().signal_details.set_label(text);
    }
}

mod imp {

    use gtk::subclass::prelude::*;
    use gtk::{CompositeTemplate, Label, glib};

    // Object holding the state
    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/signal_row.ui")]
    pub struct SignalRowImp {
        #[template_child]
        pub signal_type: TemplateChild<Label>,
        #[template_child]
        pub signal_details: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SignalRowImp {
        const NAME: &'static str = "SIGNAL_ROW";
        type Type = super::SignalRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SignalRowImp {}

    // Trait shared by all widgets
    impl WidgetImpl for SignalRowImp {}

    // Trait shared by all boxes
    impl BoxImpl for SignalRowImp {}
}
