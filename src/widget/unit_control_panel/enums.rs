#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnitContolType {
    Start,
    Stop,
    Restart,
}

impl UnitContolType {
    pub fn as_str(&self) ->&str {
        match self {
            UnitContolType::Start => "start",
            UnitContolType::Stop => "stop",
            UnitContolType::Restart => "restart",
        }
    }
}
