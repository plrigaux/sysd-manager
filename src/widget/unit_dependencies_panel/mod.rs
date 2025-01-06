mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct UnitDependenciesPanel(ObjectSubclass<imp::UnitDependenciesPanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitDependenciesPanel {
    pub fn new() -> Self {
        let obj: UnitDependenciesPanel = glib::Object::new();
        obj
    }
}
