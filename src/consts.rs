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

pub const CLASS_SUCCESS: &str = "success";
//const CLASS_ACCENT: &str = "accent";
pub const CLASS_WARNING: &str = "warning";
pub const CLASS_ERROR: &str = "error";

pub const FILTER_MARK: char = '⭐';

pub const ALL_FILTER_KEY: &str = "all";

pub const ACTION_DEFAULT_UNIT_LIST_VIEW: &str = "default_unit_list_view";
pub const NS_ACTION_DEFAULT_UNIT_LIST_VIEW: &str = concat!(WIN, ACTION_DEFAULT_UNIT_LIST_VIEW);

pub const ACTION_ACTIVE_UNIT_LIST_VIEW: &str = "active_unit_list_view";
pub const NS_ACTION_ACTIVE_UNIT_LIST_VIEW: &str = concat!(WIN, ACTION_ACTIVE_UNIT_LIST_VIEW);

pub const ACTION_UNIT_FILE_UNIT_LIST_VIEW: &str = "unit_file_unit_list_view";
pub const NS_ACTION_UNIT_FILE_UNIT_LIST_VIEW: &str = concat!(WIN, ACTION_UNIT_FILE_UNIT_LIST_VIEW);

pub const ACTION_TIMER_UNIT_LIST_VIEW: &str = "timer_unit_list_view";
pub const NS_ACTION_TIMER_UNIT_LIST_VIEW: &str = concat!(WIN, ACTION_TIMER_UNIT_LIST_VIEW);

pub const ACTION_REFRESH_UNIT_LIST: &str = "refresh_unit_list";
pub const NS_ACTION_REFRESH_UNIT_LIST: &str = concat!(WIN, ACTION_REFRESH_UNIT_LIST);
