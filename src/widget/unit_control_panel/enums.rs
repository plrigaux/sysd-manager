#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitContolType {
    Start,
    Stop,
    Restart,
}

impl UnitContolType {
    pub fn as_str(&self) -> &str {
        match self {
            UnitContolType::Start => "start",
            UnitContolType::Stop => "stop",
            UnitContolType::Restart => "restart",
        }
    }

    pub fn past_participle(&self) -> &str {
        match self {
            UnitContolType::Start => "started",
            UnitContolType::Stop => "stopped",
            UnitContolType::Restart => "restarted",
        }
    }
}
