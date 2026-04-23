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

pub const ACTION_DAEMON_RELOAD: &str = "app.daemon-reload";
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
pub const CLASS_WARNING: &str = "warning";
pub const CLASS_ERROR: &str = "error";

pub const FILTER_MARK: char = '⭐';

pub const ALL_FILTER_KEY: &str = "all";

pub const ACTION_WIN_REFRESH_UNIT_LIST: &str = "win.refresh_unit_list";

pub const ACTION_INCLUDE_UNIT_FILES: &str = "include-unit-files";
pub const WIN_ACTION_INCLUDE_UNIT_FILES: &str = concat!(WIN, ACTION_INCLUDE_UNIT_FILES);

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
// pub const TIME_LAST_TRIGGER_USEC_MONOTONIC: &str = "LastTriggerUSecMonotonic";
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
pub const ACTION_WIN_HIDE_UNIT_COL: &str = "win.hide_unit_col";
pub const ACTION_WIN_START_UNIT: &str = "win.start-unit";
pub const ACTION_WIN_STOP_UNIT: &str = "win.stop-unit";
pub const ACTION_WIN_RESTART_UNIT: &str = "win.restart-unit";
pub const ACTION_WIN_RELOAD_UNIT: &str = "win.reload-unit";
pub const ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY: &str = "win.unit_has_reload_unit_capability";
pub const ACTION_WIN_FAVORITE_SET: &str = "win.favorite-set";
pub const ACTION_WIN_FAVORITE_TOGGLE: &str = "win.favorite-toggle";
pub const ACTION_WIN_REFRESH_POP_MENU: &str = "win.refresh-pop-menu";
// pub const ACTION_WIN_COL_RESIZE: &str = "win.col_resize";
pub const ACTION_FIND_IN_TEXT_OPEN: &str = "win.find-in-text-panel";
pub const SETTING_FIND_IN_TEXT_OPEN: &str = "find-in-text-panel-open";
pub const KEY_PREF_UNIT_LIST_DISPLAY_SUMMARY: &str = "win.pref-unit-list-display-summary";
pub const UNIT_FILE_LINE_NUMBER_ACTION: &str = "win.unit-file-line-number";
