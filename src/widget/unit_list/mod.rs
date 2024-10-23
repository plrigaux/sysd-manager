use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct UnitListPanel(ObjectSubclass<imp::UnitListPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitListPanel {
    pub fn new() -> Self {
        // Create new window
        let obj: UnitListPanel = glib::Object::new();

        obj
    }


}
