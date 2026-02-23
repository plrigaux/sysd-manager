use constcat::concat;
/// 2^16-1
pub const U64MAX: u64 = 18_446_744_073_709_551_615;

pub const SUGGESTED_ACTION: &str = "suggested-action";
pub const DESTRUCTIVE_ACTION: &str = "destructive-action";
pub const FLAT: &str = "flat";

pub const ADWAITA: &str = "Adwaita";
pub const WARNING_CSS: &str = "warning";
pub const ERROR_CSS: &str = "error";

pub const APP: &str = "app.";

pub const ACTION_LIST_BOOT: &str = "list_boots";
pub const APP_ACTION_LIST_BOOT: &str = concat!(APP, ACTION_LIST_BOOT);

pub const ACTION_DAEMON_RELOAD: &str = "daemon-reload";
//pub const APP_ACTION_DAEMON_RELOAD: &str = concat!(APP, ACTION_DAEMON_RELOAD);

pub const ACTION_DAEMON_RELOAD_BUS: &str = "daemon-reload-bus";
pub const APP_ACTION_DAEMON_RELOAD_BUS: &str = concat!(APP, ACTION_DAEMON_RELOAD_BUS);

pub const ACTION_PROPERTIES_SELECTOR: &str = "properties_selector";
pub const APP_ACTION_PROPERTIES_SELECTOR: &str = concat!(APP, ACTION_PROPERTIES_SELECTOR);

pub const ACTION_PROPERTIES_SELECTOR_GENERAL: &str = "properties_selector_general";
pub const APP_ACTION_PROPERTIES_SELECTOR_GENERAL: &str =
    concat!(APP, ACTION_PROPERTIES_SELECTOR_GENERAL);

pub const WIN: &str = "win.";
pub const ACTION_UNIT_LIST_FILTER_CLEAR: &str = "unit_list_filter_clear";
pub const NS_ACTION_UNIT_LIST_FILTER_CLEAR: &str = concat!(WIN, ACTION_UNIT_LIST_FILTER_CLEAR);

pub const ACTION_UNIT_PROPERTIES_DISPLAY: &str = "unit_properties";
pub const APP_ACTION_UNIT_PROPERTIES_DISPLAY: &str = concat!(APP, ACTION_UNIT_PROPERTIES_DISPLAY);

pub const ACTION_UNIT_LIST_FILTER: &str = "unit_list_filter";
pub const NS_ACTION_UNIT_LIST_FILTER: &str = concat!(WIN, ACTION_UNIT_LIST_FILTER);

pub const MENU_ACTION: &str = "unit-reload";
pub const WIN_MENU_ACTION: &str = concat!(WIN, MENU_ACTION);

pub const ACTION_SAVE_UNIT_FILE: &str = "save-unit-file";
pub const WIN_ACTION_SAVE_UNIT_FILE: &str = concat!(WIN, ACTION_SAVE_UNIT_FILE);

pub const CLASS_SUCCESS: &str = "success";
//const CLASS_ACCENT: &str = "accent";
pub const CLASS_WARNING: &str = "warning";
pub const CLASS_ERROR: &str = "error";

pub const FILTER_MARK: char = '⭐';

pub const ALL_FILTER_KEY: &str = "all";

pub const ACTION_REFRESH_UNIT_LIST: &str = "refresh_unit_list";

pub const NS_ACTION_REFRESH_UNIT_LIST: &str = concat!(WIN, ACTION_REFRESH_UNIT_LIST);

pub const TIME_NEXT_ELAPSE_USEC_MONOTONIC: &str = "NextElapseUSecMonotonic";
pub const TIME_NEXT_ELAPSE_USEC_REALTIME: &str = "NextElapseUSecRealtime";
pub const TIMER_TIME_LAST: &str = "timerTimeLast";
pub const TIMER_TIME_PASSED: &str = "timerTimePassed";
pub const TIMER_TIME_NEXT: &str = "timerTimeNext";
pub const TIMER_TIME_LEFT: &str = "timerTimeLeft";
pub const SOCKET_LISTEN_COL: &str = "socketListen";
pub const SOCKET_LISTEN_TYPE: &str = "socketListenType";
pub const SOCKET_LISTEN: &str = "Listen";
pub const TIME_LAST_TRIGGER_USEC: &str = "LastTriggerUSec";
pub const TIME_LAST_TRIGGER_USEC_MONOTONIC: &str = "LastTriggerUSecMonotonic";
pub const SYSD_SOCKET_LISTEN: &str = "sysdSocketListen";
pub const PATH_CONDITION_COL: &str = "sysdPathCondition";
pub const PATH_PATH_COL: &str = "sysdPathPaths";
pub const PATH_PATHS: &str = "Paths";
pub const AUTOMOUNT_WHAT_COL: &str = "autoMountWhat";
pub const AUTOMOUNT_MOUNTED_COL: &str = "autoMountMounted";
pub const AUTOMOUNT_IDLE_TIMEOUT_COL: &str = "automount@TimeoutIdleUSec";
pub const AUTOMOUNT_IDLE_TIMEOUT_PROP: &str = "TimeoutIdleUSec";
pub const WHERE_PROP: &str = "Where";
pub const COL_ACTIVE: &str = "sysdm-active";
