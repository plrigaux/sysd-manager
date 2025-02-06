use gtk::glib;

pub enum JournalEventChunkInfo {
    NoMore,
    ChunkMaxReached,
    NoEventsAfterWaiting,
    Invalidate,
    Error,
}

pub struct JournalEventChunk {
    events: Vec<JournalEvent>,
    info: JournalEventChunkInfo,
}

impl JournalEventChunk {
    pub fn new(capacity: usize) -> Self {
        let events = Vec::with_capacity(capacity);

        let info = JournalEventChunkInfo::NoMore;
        JournalEventChunk { events, info }
    }

    pub fn error() -> Self {
        let events = Vec::with_capacity(0);

        let info = JournalEventChunkInfo::Error;
        JournalEventChunk { events, info }
    }

    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, JournalEvent> {
        self.events.iter()
    }

    pub fn last(&self) -> Option<&JournalEvent> {
        self.events.last()
    }

    pub fn set_info(&mut self, info: JournalEventChunkInfo) {
        self.info = info;
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

glib::wrapper! {
    pub struct JournalEvent(ObjectSubclass<imp::JournalEventImpl>);
}

impl Default for JournalEvent {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl JournalEvent {
    pub fn new_param(priority: u8, time_in_usec: u64, prefix: String, message: String) -> Self {
        let obj: JournalEvent = glib::Object::new();
        obj.set_prefix(prefix);
        obj.set_message(message);
        obj.set_timestamp(time_in_usec);
        obj.set_priority(priority);
        obj
    }
}

mod imp {
    use std::sync::RwLock;

    use gtk::{
        glib::{self, Object},
        prelude::*,
        subclass::prelude::*,
    };

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
