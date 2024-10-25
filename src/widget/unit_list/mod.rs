use gtk::glib;
use gtk::subclass::prelude::*;
use super::app_window::AppWindow;

mod imp;

glib::wrapper! {
    pub struct UnitListPanel(ObjectSubclass<imp::UnitListPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitListPanel {
    pub fn register_selection_change(&self, app_window : &AppWindow) {
        let obj = self.imp();
        obj.register_selection_change(app_window);
    }

    pub fn search_bar(&self) -> gtk::SearchBar {
        self.imp().search_bar()
    }
}
