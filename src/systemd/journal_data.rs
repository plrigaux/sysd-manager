#[derive(Clone, Copy, Debug)]
pub enum JournalEventChunkInfo {
    NoMore,
    ChunkMaxReached,
    //NoEventsAfterWaiting,
    //Invalidate,
    Tail,
    Error,
}

#[derive(Debug)]
pub struct JournalEventChunk {
    events: Vec<JournalEvent>,
    pub info: JournalEventChunkInfo,
    pub what_grab: WhatGrab,
}

impl JournalEventChunk {
    pub fn new(capacity: usize, what_grab: WhatGrab) -> Self {
        Self::new_info(capacity, JournalEventChunkInfo::NoMore, what_grab)
    }

    pub fn new_info(capacity: usize, info: JournalEventChunkInfo, what_grab: WhatGrab) -> Self {
        let events = Vec::with_capacity(capacity);

        JournalEventChunk {
            events,
            info,
            what_grab,
        }
    }

    pub fn error(what_grab: WhatGrab) -> Self {
        let events = Vec::with_capacity(0);

        let info = JournalEventChunkInfo::Error;
        JournalEventChunk {
            events,
            info,
            what_grab,
        }
    }

    pub fn push(&mut self, event: JournalEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
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

    pub fn times(&self) -> Option<(u64, u64)> {
        if self.events.is_empty() {
            return None;
        }

        let first = self.events.first().expect("must have a first");
        let last = self.events.last().expect("must have a last");

        match self.what_grab {
            WhatGrab::Newer => Some((first.timestamp, last.timestamp)),
            WhatGrab::Older => Some((last.timestamp, first.timestamp)),
        }
    }

    /*     pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    } */
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WhatGrab {
    //Grab newer events
    Newer,
    //Grab older events
    Older,
}

#[derive(Debug)]
pub struct EventRange {
    pub oldest_events_time: Option<u64>,
    pub newest_events_time: Option<u64>,
    pub batch_size: usize,
    pub what_grab: WhatGrab,
}

impl EventRange {
    pub fn new(
        what_grab: WhatGrab,
        batch_size: usize,
        oldest_events_time: Option<u64>,
        newest_events_time: Option<u64>,
    ) -> Self {
        EventRange {
            oldest_events_time,
            newest_events_time,
            batch_size,
            what_grab,
        }
    }

    /*     pub fn has_reached_end(&self, time: u64) -> bool {
        if let Some(end) = self.end {
            if self.oldest_first {
                time >= end
            } else {
                time <= end
            }
        } else {
            false
        }
    } */
}

#[derive(Debug)]
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
