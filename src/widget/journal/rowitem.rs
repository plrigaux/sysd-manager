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
    pub fn new(event_timestamp : u64, message: String) -> Self {

            let obj: JournalEvent = glib::Object::new();
            obj.set_message(message);
            obj.set_timestamp(event_timestamp);
    
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for JournalEventImpl {
        const NAME: &'static str = "JournalEvent";
        type Type = super::JournalEvent;
    }

    #[glib::derived_properties]
    impl ObjectImpl for JournalEventImpl {}
}
