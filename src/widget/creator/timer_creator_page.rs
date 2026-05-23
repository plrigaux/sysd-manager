mod imp;
use crate::widget::creator::UnitCreatorWindow;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

glib::wrapper! {

    pub struct TimerCreatorPage(ObjectSubclass<imp::TimerCreatorPageImp>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget ;
}

impl TimerCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>) -> Self {
        let obj: TimerCreatorPage = glib::Object::new();
        let _ = obj.imp().window.set(window);
        obj.imp().update_from_unit_info();
        obj.imp().create_actions();
        obj
    }
}
