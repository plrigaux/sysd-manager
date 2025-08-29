use constcat::concat;
/// 2^16-1
pub const U64MAX: u64 = 18_446_744_073_709_551_615;

pub const SUGGESTED_ACTION: &str = "suggested-action";
pub const DESTRUCTIVE_ACTION: &str = "destructive-action";

pub const ADWAITA: &str = "Adwaita";
pub const WARNING_CSS: &str = "warning";
pub const ERROR_CSS: &str = "error";

pub const ACTION_LIST_BOOT: &str = "list_boots";
pub const APP_ACTION_LIST_BOOT: &str = concat!("app.", ACTION_LIST_BOOT);

pub const WIN: &str = "win.";
pub const ACTION_UNIT_LIST_FILTER_CLEAR: &str = "unit_list_filter_clear";
pub const NS_ACTION_UNIT_LIST_FILTER_CLEAR: &str = concat!(WIN, ACTION_UNIT_LIST_FILTER_CLEAR);

pub const MENU_ACTION: &str = "unit-reload";
pub const WIN_MENU_ACTION: &str = concat!(WIN, MENU_ACTION);
