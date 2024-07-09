mod imp;

use crate::gtk::{glib, prelude::*, subclass::prelude::*};

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

impl ExMenuButton {
    pub fn new(label: &str) -> Self {
        let obj = Self::default();
        obj.set_button_label(label);

        obj
    }

    pub fn set_button_label(&self, label: &str) {
        self.imp().button_label.set_label(label);
    }

    pub fn add_item(&self, label: &str) {
        let check = gtk::CheckButton::with_label(label);
        self.imp().pop_content.append(&check);
    }
}
