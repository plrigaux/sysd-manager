use gettextrs::pgettext;
use glib::value::ToValue;
use strum::EnumIter;
use tracing::warn;

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default, EnumIter, Hash, Ord, PartialOrd,
)]
#[enum_type(name = "UnitDBusLevel")]
pub enum UnitDBusLevel {
    #[default]
    #[enum_value(name = "system", nick = "System")]
    System = 0,
    #[enum_value(name = "user", nick = "User Session")]
    UserSession = 1,
    #[enum_value(name = "both", nick = "System & User")]
    Both = 2,
}

impl UnitDBusLevel {
    pub fn short(&self) -> &str {
        match self {
            UnitDBusLevel::System => "s",
            UnitDBusLevel::UserSession => "u",
            UnitDBusLevel::Both => "b",
        }
    }

    pub fn as_str(&self) -> &'static str {
        let level_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        level_value.name()
    }

    //used in browser table
    pub fn label(&self) -> &'static str {
        self.as_str()
    }

    pub fn nice_label(&self) -> String {
        match self {
            //menu option
            UnitDBusLevel::UserSession => pgettext("dbus", "User Session"),
            //menu option
            _ => pgettext("dbus", "System"),
        }
    }

    pub fn from_short(suffix: &str) -> Self {
        match suffix {
            "s" => UnitDBusLevel::System,
            "u" => UnitDBusLevel::UserSession,
            "b" => UnitDBusLevel::Both,
            _ => UnitDBusLevel::System,
        }
    }

    pub fn tooltip_info(&self) -> Option<&str> {
        None
    }

    pub fn value(&self) -> i32 {
        let level_value: &glib::EnumValue = self.to_value().get().expect("it's an enum");
        level_value.value()
    }

    pub fn index(&self) -> u8 {
        match self {
            UnitDBusLevel::System => 0,
            UnitDBusLevel::UserSession => 1,
            UnitDBusLevel::Both => 2,
        }
    }
}

impl From<u8> for UnitDBusLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => UnitDBusLevel::System,
            1 => UnitDBusLevel::UserSession,
            2 => UnitDBusLevel::Both,
            _ => UnitDBusLevel::UserSession,
        }
    }
}

impl From<u32> for UnitDBusLevel {
    fn from(level: u32) -> Self {
        (level as u8).into()
    }
}

impl From<&str> for UnitDBusLevel {
    fn from(level: &str) -> Self {
        match level {
            "user" => UnitDBusLevel::UserSession,
            "system" => UnitDBusLevel::System,
            _ => {
                warn!("Unit dbus Level not found {level:?}");
                UnitDBusLevel::default()
            }
        }
    }
}
