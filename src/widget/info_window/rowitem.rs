use gtk::glib;

glib::wrapper! {
    pub struct Metadata(ObjectSubclass<imp::Metadata>);
}

impl Default for Metadata {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Metadata {
    pub fn new(col0: u32, col1: String, col2: String) -> Self {
        glib::Object::builder()
            .property("col0", col0)
            .property("col1", col1)
            .property("col2", col2)
            .build()
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Metadata)]
    pub struct Metadata {
        #[property(get, set)]
        pub col0: Cell<u32>,
        #[property(get, set)]
        pub col1: RefCell<String>,
        #[property(get, set)]
        pub col2: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Metadata {
        const NAME: &'static str = "Metadata";
        type Type = super::Metadata;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Metadata {}
}
