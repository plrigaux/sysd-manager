use gtk::glib;

use crate::systemd::JournalEventRaw;

glib::wrapper! {
    pub struct JournalEvent(ObjectSubclass<imp::JournalEventImpl>);
}

impl Default for JournalEvent {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl JournalEvent {
    pub fn new(event : JournalEventRaw) -> Self {

            let obj: JournalEvent = glib::Object::new();
            obj.set_message(event.message);
            obj.set_timestamp(event.time);
            obj.set_priority(event.priority);
    
            obj
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::JournalEvent)]
    pub struct JournalEventImpl {
        #[property(get, set)]
        pub message: RefCell<String>,
        
        #[property(get, set)]
        pub timestamp: Cell<u64>, 


        #[property(get, set)]
        pub priority: Cell<u8>, 
    }

    #[glib::object_subclass]
    impl ObjectSubclass for JournalEventImpl {
        const NAME: &'static str = "JournalEvent";
        type Type = super::JournalEvent;
    }

    #[glib::derived_properties]
    impl ObjectImpl for JournalEventImpl {}
}
