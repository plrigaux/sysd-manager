mod imp;

use crate::gtk::{glib, subclass::prelude::*};
use std::collections::HashMap;

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

        let imp = obj.imp();
        imp.check_boxes.replace(HashMap::new());

        obj
    }

    pub fn set_button_label(&self, label: &str) {
        self.imp().button_label.set_label(label);
    }

    pub fn add_item(&mut self, label: &str) {
        let binding = self.imp();

        binding.add_item(label);
    }
}
