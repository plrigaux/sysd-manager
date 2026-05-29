use gtk::glib::{self};

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
}

mod imp {

    use super::*;
    use adw::subclass::prelude::*;
    use gtk::glib;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/nav_row.ui")]
    #[properties(wrapper_type = super::NavigationRow)]
    pub struct NavigationRowImp {}

    impl NavigationRowImp {}

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
