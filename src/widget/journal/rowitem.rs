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
    pub fn new(col1: String) -> Self {

            let obj: JournalEvent = glib::Object::new();
            obj.set_col1(col1);
         //   obj.set_col2(col2);
    
            obj
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::JournalEvent)]
    pub struct JournalEventImpl {
        #[property(get, set)]
        pub col1: RefCell<String>,
/*         #[property(get, set)]
        pub col2: RefCell<String>, */
    }

    #[glib::object_subclass]
    impl ObjectSubclass for JournalEventImpl {
        const NAME: &'static str = "JournalEvent";
        type Type = super::JournalEvent;
    }

    #[glib::derived_properties]
    impl ObjectImpl for JournalEventImpl {}
}
