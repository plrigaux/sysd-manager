use gtk::glib;

glib::wrapper! {
    pub struct JournalEvent(ObjectSubclass<imp::JournalEventImpl>);
}

impl Default for JournalEvent {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl JournalEvent {
    pub fn new_param(priority: u8, time: u64, prefix: String, message: String) -> Self {
        let obj: JournalEvent = glib::Object::new();
        obj.set_prefix(prefix);
        obj.set_message(message);
        obj.set_timestamp(time);
        obj.set_priority(priority);
        obj
    }
}

mod imp {
    use std::sync::RwLock;

    use gtk::{glib::{self, Object}, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::JournalEvent)]
    pub struct JournalEventImpl {
        #[property(get, set)]
        pub prefix: RwLock<String>,

        #[property(get, set)]
        pub message: RwLock<String>,

        #[property(get, set)]
        pub timestamp: RwLock<u64>,

        #[property(get, set)]
        pub priority: RwLock<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for JournalEventImpl {
        const NAME: &'static str = "JournalEvent";
        type Type = super::JournalEvent;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for JournalEventImpl {}
}
