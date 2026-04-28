use gtk::glib::{self};

glib::wrapper! {

    pub struct ServiceCreatorPage(ObjectSubclass<imp::ServiceCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl ServiceCreatorPage {
    pub fn new() -> Self {
        let obj: ServiceCreatorPage = glib::Object::new();
        obj
    }
}

impl Default for ServiceCreatorPage {
    fn default() -> Self {
        ServiceCreatorPage::new()
    }
}

mod imp {

    use super::*;
    use adw::subclass::prelude::*;
    use gtk::glib::{self};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/service_creator_page.ui")]
    pub struct ServiceCreatorPageImp {}

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceCreatorPageImp {
        const NAME: &'static str = "ServiceCreatorPage";
        type Type = ServiceCreatorPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            //klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServiceCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl ServiceCreatorPageImp {}

    impl WidgetImpl for ServiceCreatorPageImp {}

    impl NavigationPageImpl for ServiceCreatorPageImp {}
}
