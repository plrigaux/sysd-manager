mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};

use super::app_window::AppWindow;

// ANCHOR: mod
glib::wrapper! {
    pub struct SignalsWindow(ObjectSubclass<imp::SignalsWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl SignalsWindow {
    pub fn new(app_window: Option<&AppWindow>) -> Self {
        let obj: SignalsWindow = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window);

        obj
    }
}
