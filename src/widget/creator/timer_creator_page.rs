use gtk::glib::{self};

glib::wrapper! {

    pub struct TimerCreatorPage(ObjectSubclass<imp::TimerCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl TimerCreatorPage {
    pub fn new() -> Self {
        let obj: TimerCreatorPage = glib::Object::new();
        obj
    }
}

impl Default for TimerCreatorPage {
    fn default() -> Self {
        TimerCreatorPage::new()
    }
}

mod imp {

    use super::TimerCreatorPage;
    use adw::subclass::prelude::*;
    use gtk::glib::{self};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/timer_creator_page.ui")]
    pub struct TimerCreatorPageImp {}

    #[glib::object_subclass]
    impl ObjectSubclass for TimerCreatorPageImp {
        const NAME: &'static str = "TimerCreatorPage";
        type Type = TimerCreatorPage;
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

    impl ObjectImpl for TimerCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl TimerCreatorPageImp {}

    impl WidgetImpl for TimerCreatorPageImp {}

    impl NavigationPageImpl for TimerCreatorPageImp {}
}
