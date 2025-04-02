mod imp;

use gtk::glib::{self};

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitListFilterWindow(ObjectSubclass<imp::UnitListFilterWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitListFilterWindow {
    pub fn new(selected_filter: Option<String>) -> Self {
        let obj: UnitListFilterWindow = glib::Object::builder()
            .property("selected", selected_filter)
            .build();
        obj
    }
}
