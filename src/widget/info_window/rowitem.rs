use gtk::glib;
use systemd::enums::UnitType;

glib::wrapper! {
    pub struct Metadata(ObjectSubclass<imp::Metadata>);
}

impl Default for Metadata {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Metadata {
    pub fn new(index: u32, unit_type: UnitType, col1: String, col2: String, empty: bool) -> Self {
        glib::Object::builder()
            .property("index", index)
            .property("unit_prop", col1)
            .property("prop_value", col2)
            .property("unit_type", unit_type)
            .property("is_empty", empty)
            .build()
    }
}

mod imp {
    use std::cell::{Cell, OnceCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};
    use systemd::enums::UnitType;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Metadata)]
    pub struct Metadata {
        #[property(get, set)]
        pub index: Cell<u32>,
        #[property(get, set)]
        pub unit_prop: OnceCell<String>,
        #[property(get, set)]
        pub prop_value: OnceCell<String>,
        #[property(get, set, default)]
        pub unit_type: OnceCell<UnitType>,
        #[property(get, set)]
        pub is_empty: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Metadata {
        const NAME: &'static str = "Metadata";
        type Type = super::Metadata;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Metadata {}
}
