use gettextrs::pgettext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitContolType {
    Start,
    Stop,
    Restart,
}

impl UnitContolType {
    pub fn as_str(&self) -> String {
        match self {
            UnitContolType::Start => pgettext("toast", "start"),
            UnitContolType::Stop => pgettext("toast", "stop"),
            UnitContolType::Restart => pgettext("toast", "restart"),
        }
    }

    pub fn past_participle(&self) -> String {
        match self {
            UnitContolType::Start => pgettext("toast", "started"),
            UnitContolType::Stop => pgettext("toast", "stopped"),
            UnitContolType::Restart => pgettext("toast", "restarted"),
        }
    }
}
