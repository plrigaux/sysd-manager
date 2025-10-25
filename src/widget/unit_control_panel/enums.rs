use gettextrs::pgettext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitContolType {
    Start,
    Stop,
    Restart,
    Reload,
}

impl UnitContolType {
    pub fn code(&self) -> &str {
        match self {
            UnitContolType::Start => "start",
            UnitContolType::Stop => "stop",
            UnitContolType::Restart => "restart",
            UnitContolType::Reload => "reload",
        }
    }
    pub fn label(&self) -> String {
        match self {
            //unit action in toast message
            UnitContolType::Start => pgettext("toast", "start"),
            //unit action in toast message
            UnitContolType::Stop => pgettext("toast", "stop"),
            //unit action in toast message
            UnitContolType::Restart => pgettext("toast", "restart"),
            //unit action in toast message
            UnitContolType::Reload => pgettext("toast", "reload"),
        }
    }

    pub fn past_participle(&self) -> String {
        match self {
            //unit action in toast message
            UnitContolType::Start => pgettext("toast", "started"),
            //unit action in toast message
            UnitContolType::Stop => pgettext("toast", "stopped"),
            //unit action in toast message
            UnitContolType::Restart => pgettext("toast", "restarted"),
            //unit action in toast message
            UnitContolType::Reload => pgettext("toast", "reloaded"),
        }
    }
}
