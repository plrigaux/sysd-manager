use glib::GString;
use systemd::enums::UnitType;
use tracing::error;

use crate::{
    consts::{
        AUTOMOUNT_IDLE_TIMEOUT_COL, AUTOMOUNT_IDLE_TIMEOUT_PROP, AUTOMOUNT_MOUNTED_COL,
        AUTOMOUNT_WHAT_COL, COL_ACTIVE, PATH_CONDITION_COL, PATH_PATH_COL, PATH_PATHS,
        SOCKET_LISTEN, SOCKET_LISTEN_COL, SOCKET_LISTEN_TYPE, TIME_LAST_TRIGGER_USEC,
        TIME_NEXT_ELAPSE_USEC_MONOTONIC, TIME_NEXT_ELAPSE_USEC_REALTIME, TIMER_TIME_LAST,
        TIMER_TIME_LEFT, TIMER_TIME_NEXT, TIMER_TIME_PASSED, WHERE_PROP,
    },
    widget::{
        unit_list::{COL_ID_UNIT, COL_ID_UNIT_FULL},
        unit_properties_selector::{
            data_browser::{INTERFACE_NAME, SPECIAL_INTERFACE_NAME},
            save::UnitColumn,
        },
    },
};

const COL_BUS: &str = "sysdm-bus";
const COL_PRESET: &str = "sysdm-preset";
const COL_STATE: &str = "sysdm-state";
const COL_TYPE: &str = "sysdm-type";
const COL_SUBSTATE: &str = "sysdm-sub";
const COL_LOAD: &str = "sysdm-load";
const COL_DESCRIPTION: &str = "sysdm-description";

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct CustomProp {
    utype: UnitType,
    id: String,
    prop_idx: u8,
    signature: Option<String>,
}

impl CustomProp {
    fn property(&self) -> &str {
        &self.id[(self.prop_idx as usize)..]
    }
}

#[derive(Debug, Eq, Clone)]
pub enum SysdColumn {
    Name,
    FullName,
    Bus,
    Type,
    State,
    Preset,
    Load,
    Active,
    SubState,
    Description,
    TimerTimeNextElapseRT,
    TimerTimeLeftElapseMono,
    TimerTimePassed,
    TimerTimeLast,
    SocketListen,
    SocketListenType,
    PathCondition,
    Path,
    AutomountWhat,
    AutomountMounted,
    AutomountIdleTimeOut,
    Custom(CustomProp),
}

impl SysdColumn {
    pub(crate) fn new_from_props(
        property_name: &str,
        interface: &str,
        signature: Option<String>,
    ) -> SysdColumn {
        // let signature = signature.map(|s| s.to_owned());

        let r = match interface {
            "" | SPECIAL_INTERFACE_NAME | INTERFACE_NAME => {
                Self::new(property_name.to_owned(), signature)
            }
            _ => {
                let ut = UnitType::from_intreface(interface);
                let new_id = format!("{}@{property_name}", ut.as_str());
                Self::new(new_id, signature)
            }
        };

        match r {
            Ok(a) => a,
            Err(e) => e.1,
        }
    }

    pub fn new(
        id: String,
        prop_type: Option<String>,
    ) -> Result<SysdColumn, (SysdColumnNonConformity, SysdColumn)> {
        let col = match id.as_str() {
            COL_ID_UNIT => SysdColumn::Name,
            COL_ID_UNIT_FULL => SysdColumn::FullName,
            COL_BUS => SysdColumn::Bus,
            COL_TYPE => SysdColumn::Type,
            COL_STATE => SysdColumn::State,
            COL_PRESET => SysdColumn::Preset,
            COL_LOAD => SysdColumn::Load,
            COL_ACTIVE => SysdColumn::Active,
            COL_SUBSTATE => SysdColumn::Bus,
            COL_DESCRIPTION => SysdColumn::Description,
            TIMER_TIME_NEXT => SysdColumn::TimerTimeNextElapseRT,
            TIMER_TIME_LEFT => SysdColumn::TimerTimeLeftElapseMono,
            TIMER_TIME_PASSED => SysdColumn::TimerTimePassed,
            TIMER_TIME_LAST => SysdColumn::TimerTimeLast,
            SOCKET_LISTEN_COL => SysdColumn::SocketListen,
            SOCKET_LISTEN_TYPE => SysdColumn::SocketListenType,
            PATH_CONDITION_COL => SysdColumn::PathCondition,
            PATH_PATH_COL => SysdColumn::Path,
            AUTOMOUNT_MOUNTED_COL => SysdColumn::AutomountMounted,
            AUTOMOUNT_WHAT_COL => SysdColumn::AutomountWhat,
            AUTOMOUNT_IDLE_TIMEOUT_COL => SysdColumn::AutomountIdleTimeOut,
            _ => {
                if let Some((utype, _prop)) = id.split_once('@') {
                    let ut: UnitType = utype.into();
                    if UnitType::Unknown != ut {
                        if verify_prop_type(&prop_type) {
                            return Ok(Self::fill_custom2(ut, id, prop_type));
                        } else {
                            return Err((
                                SysdColumnNonConformity::CustomColIdWithoutUnitType,
                                defective_custom(id, prop_type),
                            ));
                        }
                    } else {
                        return Err((
                            SysdColumnNonConformity::CustomColWithoutDefinePropType,
                            defective_custom(id, prop_type),
                        ));
                    }
                } else {
                    error!("Unknown type for property : {:?}", id);
                    return Err((
                        SysdColumnNonConformity::CustomColWithoutDefinePropType,
                        defective_custom(id, prop_type),
                    ));
                }
            }
        };
        Ok(col)
    }

    pub fn id(&self) -> &str {
        match self {
            SysdColumn::Name => COL_ID_UNIT,
            SysdColumn::FullName => COL_ID_UNIT_FULL,
            SysdColumn::Bus => COL_BUS,
            SysdColumn::Type => COL_TYPE,
            SysdColumn::State => COL_STATE,
            SysdColumn::Preset => COL_PRESET,
            SysdColumn::Load => COL_LOAD,
            SysdColumn::Active => COL_ACTIVE,
            SysdColumn::SubState => COL_SUBSTATE,
            SysdColumn::Description => COL_DESCRIPTION,
            SysdColumn::TimerTimeNextElapseRT => TIMER_TIME_NEXT,
            SysdColumn::TimerTimeLeftElapseMono => TIMER_TIME_LEFT,
            SysdColumn::TimerTimePassed => TIMER_TIME_PASSED,
            SysdColumn::TimerTimeLast => TIMER_TIME_LAST,
            SysdColumn::SocketListen => SOCKET_LISTEN_COL,
            SysdColumn::SocketListenType => SOCKET_LISTEN_TYPE,
            SysdColumn::PathCondition => PATH_CONDITION_COL,
            SysdColumn::Path => PATH_PATH_COL,
            SysdColumn::AutomountWhat => AUTOMOUNT_WHAT_COL,
            SysdColumn::AutomountMounted => AUTOMOUNT_MOUNTED_COL,
            SysdColumn::AutomountIdleTimeOut => AUTOMOUNT_IDLE_TIMEOUT_COL,
            SysdColumn::Custom(c) => c.id.as_str(),
        }
    }

    pub fn property_type(&self) -> &Option<String> {
        match self {
            SysdColumn::Custom(c) => &c.signature,
            _ => &None,
        }
    }

    pub(crate) fn property(&self) -> &str {
        match self {
            SysdColumn::Name => "Name",
            SysdColumn::FullName => "Name",
            SysdColumn::Type => "Type",
            SysdColumn::Bus => "Bus",
            SysdColumn::State => "UnitFileState",
            SysdColumn::Preset => "UnitFilePreset",
            SysdColumn::Load => "LoadStat",
            SysdColumn::Active => "Active",
            SysdColumn::SubState => "SubState",
            SysdColumn::Description => "Description",
            SysdColumn::TimerTimePassed | SysdColumn::TimerTimeLast => TIME_LAST_TRIGGER_USEC,
            SysdColumn::TimerTimeNextElapseRT => TIME_NEXT_ELAPSE_USEC_REALTIME,
            SysdColumn::TimerTimeLeftElapseMono => TIME_NEXT_ELAPSE_USEC_MONOTONIC,
            SysdColumn::SocketListen | SysdColumn::SocketListenType => SOCKET_LISTEN,
            SysdColumn::PathCondition | SysdColumn::Path => PATH_PATHS,
            SysdColumn::AutomountMounted | SysdColumn::AutomountWhat => WHERE_PROP,
            SysdColumn::AutomountIdleTimeOut => AUTOMOUNT_IDLE_TIMEOUT_PROP,
            SysdColumn::Custom(c) => c.property(),
        }
    }

    pub(crate) fn generate_quark(&self) -> glib::Quark {
        let qstr = self.property();
        glib::Quark::from_str(qstr)
    }

    pub(crate) fn utype(&self) -> UnitType {
        match self {
            SysdColumn::TimerTimePassed
            | SysdColumn::TimerTimeLast
            | SysdColumn::TimerTimeNextElapseRT
            | SysdColumn::TimerTimeLeftElapseMono => UnitType::Timer,
            SysdColumn::SocketListen | SysdColumn::SocketListenType => UnitType::Socket,
            SysdColumn::PathCondition | SysdColumn::Path => UnitType::Path,
            SysdColumn::AutomountWhat
            | SysdColumn::AutomountMounted
            | SysdColumn::AutomountIdleTimeOut => UnitType::Automount,
            SysdColumn::Custom(c) => c.utype,
            _ => UnitType::Unknown,
        }
    }

    pub(crate) fn is_custom(&self) -> bool {
        matches!(self, SysdColumn::Custom(_,))
    }

    pub(crate) fn verify(
        unit_column_config: &UnitColumn,
    ) -> Result<SysdColumn, (SysdColumnNonConformity, SysdColumn)> {
        Self::new(
            unit_column_config.id.clone(),
            unit_column_config.prop_type.clone(),
        )
    }

    pub(crate) fn fill_custom(utype: UnitType, property: &str, property_type: &str) -> SysdColumn {
        let utype_str = utype.as_str();
        let id = format!("{}@{property}", utype_str);
        let c = CustomProp {
            utype,
            id,
            prop_idx: (utype_str.len() + 1) as u8,
            signature: Some(property_type.to_owned()),
        };
        SysdColumn::Custom(c)
    }

    fn fill_custom2(utype: UnitType, id: String, signature: Option<String>) -> SysdColumn {
        let utype_str = utype.as_str();
        let c = CustomProp {
            utype,
            id,
            prop_idx: (utype_str.len() + 1) as u8,
            signature,
        };
        SysdColumn::Custom(c)
    }

    pub(crate) fn specials() -> Vec<SysdColumn> {
        vec![
            SysdColumn::TimerTimeNextElapseRT,
            SysdColumn::TimerTimeLeftElapseMono,
            SysdColumn::TimerTimePassed,
            SysdColumn::TimerTimeLast,
            SysdColumn::SocketListen,
            SysdColumn::SocketListenType,
            SysdColumn::PathCondition,
            SysdColumn::Path,
            SysdColumn::AutomountWhat,
            SysdColumn::AutomountMounted,
            SysdColumn::AutomountIdleTimeOut,
        ]
    }
}

impl std::hash::Hash for SysdColumn {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self {
            SysdColumn::SocketListen | SysdColumn::SocketListenType => {
                core::mem::discriminant(&SysdColumn::SocketListen).hash(state)
            }
            _ => core::mem::discriminant(self).hash(state),
        }
    }
}

impl PartialEq for SysdColumn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::SocketListen | Self::SocketListenType,
                Self::SocketListen | Self::SocketListenType,
            ) => true,
            (Self::Custom(l0), Self::Custom(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

fn defective_custom(id: String, signature: Option<String>) -> SysdColumn {
    SysdColumn::Custom(CustomProp {
        utype: UnitType::Unknown,
        id,
        prop_idx: 0,
        signature,
    })
}

fn verify_prop_type(prop_type: &Option<String>) -> bool {
    prop_type.is_some()
}

impl From<(&str, Option<String>)> for SysdColumn {
    fn from(value: (&str, Option<String>)) -> Self {
        (value.0.to_owned(), value.1).into()
    }
}

impl From<(String, Option<String>)> for SysdColumn {
    fn from(value: (String, Option<String>)) -> Self {
        match SysdColumn::new(value.0, value.1) {
            Ok(c) => c,
            Err((_e, c)) => c,
        }
    }
}

impl From<(GString, Option<String>)> for SysdColumn {
    fn from(value: (GString, Option<String>)) -> Self {
        (value.0.as_str(), value.1).into()
    }
}

#[derive(Debug)]
pub enum SysdColumnNonConformity {
    CustomColWithoutDefinePropType,
    CustomColIdWithoutUnitType,
}
