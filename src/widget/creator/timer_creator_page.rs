mod imp;
use crate::widget::creator::{UnitCreatorWindow, unit_file_creator_page::UnitFileCreatorPage};
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

    pub fn update_from_unit_info(&self) {
        self.imp().update_from_unit_info();
    }

    pub fn update_view(&self, page: &UnitFileCreatorPage) {
        self.imp().update_view(page);
    }

    pub fn update_file_data(&self, content: &str) {
        self.imp().update_file_data(content);
    }
}
