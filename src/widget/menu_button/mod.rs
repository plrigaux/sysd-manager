mod imp;


use gtk::glib;

glib::wrapper! {
    pub struct ExMenuButton(ObjectSubclass<imp::ExMenuButton>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}


impl Default for ExMenuButton {
    fn default() -> Self {
        glib::Object::new()
    }
}