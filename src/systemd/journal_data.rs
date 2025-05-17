#[derive(Clone, Copy, Debug)]
pub enum JournalEventChunkInfo {
    NoMore,
    ChunkMaxReached,
    //NoEventsAfterWaiting,
    //Invalidate,
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

    pub fn info(&self) -> JournalEventChunkInfo {
        self.info
    }

    pub fn first(&self) -> Option<&JournalEvent> {
        self.events.first()
    }

    /*     pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    } */
}

#[derive(Default, Debug)]
pub struct EventRange {
    pub begin: Option<u64>,
    pub end: Option<u64>,
    pub batch_size: usize,
    pub oldest_first: bool,
}

impl EventRange {
    pub fn basic(oldest_first: bool, max: usize, begin: Option<u64>) -> Self {
        EventRange {
            oldest_first,
            batch_size: max,
            begin,
            end: None,
        }
    }

    pub fn decending(&self) -> bool {
        !self.oldest_first
    }

    pub fn has_reached_end(&self, time: u64) -> bool {
        if let Some(end) = self.end {
            if self.oldest_first {
                time >= end
            } else {
                time <= end
            }
        } else {
            false
        }
    }
}

pub struct JournalEvent {
    pub prefix: String,
    pub message: String,
    pub timestamp: u64,
    pub priority: u8,
}

impl JournalEvent {
    pub fn new_param(priority: u8, timestamp: u64, prefix: String, message: String) -> Self {
        JournalEvent {
            prefix,
            message,
            timestamp,
            priority,
        }
    }
}
