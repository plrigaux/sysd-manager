use glib::GString;
use systemd::enums::UnitType;

use crate::{
    consts::{
        AUTOMOUNT_IDLE_TIMEOUT_COL, AUTOMOUNT_IDLE_TIMEOUT_PROP, AUTOMOUNT_MOUNTED_COL,
        AUTOMOUNT_WHAT_COL, COL_ACTIVE, PATH_CONDITION_COL, PATH_PATH_COL, PATH_PATHS,
        SOCKET_LISTEN, SOCKET_LISTEN_COL, SOCKET_LISTEN_TYPE, TIME_LAST_TRIGGER_USEC,
        TIME_NEXT_ELAPSE_USEC_MONOTONIC, TIME_NEXT_ELAPSE_USEC_REALTIME, TIMER_TIME_LAST,
        TIMER_TIME_LEFT, TIMER_TIME_NEXT, TIMER_TIME_PASSED, WHERE_PROP,
    },
    widget::unit_list::{COL_ID_UNIT, COL_ID_UNIT_FULL},
};

const COL_BUS: &str = "sysdm-bus";
const COL_PRESET: &str = "sysdm-preset";
const COL_STATE: &str = "sysdm-state";
const COL_TYPE: &str = "sysdm-type";
const COL_SUBSTATE: &str = "sysdm-sub";
const COL_LOAD: &str = "sysdm-load";
const COL_DESCRIPTION: &str = "sysdm-description";

#[derive(Debug, Eq, PartialEq, Hash)]
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
    Custom(UnitType, String, Option<String>),
}

impl SysdColumn {
    pub fn new_custom(utype: UnitType, id: String, prop: Option<String>) -> Self {
        SysdColumn::Custom(utype, id, prop)
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
            SysdColumn::Custom(_, id, _) => id.as_str(),
        }
    }

    pub fn property_type(&self) -> &Option<String> {
        match self {
            SysdColumn::Custom(_, _, p) => p,
            _ => &None,
        }
    }

    pub(crate) fn property(&self) -> &str {
        match self {
            SysdColumn::TimerTimePassed | SysdColumn::TimerTimeLast => TIME_LAST_TRIGGER_USEC,
            SysdColumn::TimerTimeNextElapseRT => TIME_NEXT_ELAPSE_USEC_REALTIME,
            SysdColumn::TimerTimeLeftElapseMono => TIME_NEXT_ELAPSE_USEC_MONOTONIC,
            SysdColumn::SocketListen | SysdColumn::SocketListenType => SOCKET_LISTEN,
            SysdColumn::PathCondition | SysdColumn::Path => PATH_PATHS,
            SysdColumn::AutomountMounted | SysdColumn::AutomountWhat => WHERE_PROP,
            SysdColumn::AutomountIdleTimeOut => AUTOMOUNT_IDLE_TIMEOUT_PROP,
            SysdColumn::Custom(_, id, _) => id.as_str(),
            _ => unreachable!("Need to define a property for: {:?}", self),
        }
    }

    pub(crate) fn generate_quark(&self) -> glib::Quark {
        // let qstr = match self {
        //     SysdColumn::Path | SysdColumn::PathCondition => PATH_PATH_COL,
        //     _ => self.property(),
        // };

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
            SysdColumn::Custom(utype, _, _) => *utype,
            _ => UnitType::Unknown,
        }
    }

    pub(crate) fn is_custom(&self) -> bool {
        matches!(self, SysdColumn::Custom(_, _, _))
    }
}

impl From<(&str, Option<String>)> for SysdColumn {
    fn from(value: (&str, Option<String>)) -> Self {
        match value.0 {
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
                if let Some((utype, prop)) = value.0.split_once('@') {
                    let ut: UnitType = utype.into();
                    SysdColumn::Custom(ut, prop.to_owned(), value.1)
                } else {
                    SysdColumn::Custom(UnitType::Unknown, value.0.to_string(), value.1)
                }
            }
        }
    }
}

impl From<(GString, Option<String>)> for SysdColumn {
    fn from(value: (GString, Option<String>)) -> Self {
        (value.0.as_str(), value.1).into()
    }
}
