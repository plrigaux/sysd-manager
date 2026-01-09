use std::sync::RwLock;

use base::consts::APP_ID;
use gtk::gio::Settings;
use tracing::error;

pub fn new_settings() -> Settings {
    Settings::new(APP_ID)
}

static IS_DARK: RwLock<bool> = RwLock::new(false);

pub fn set_is_dark(is_dark: bool) {
    match IS_DARK.write() {
        Ok(mut d) => *d = is_dark,
        Err(err) => {
            error!("Poisoned {err:?}")
        }
    }
}

pub fn is_dark() -> bool {
    match IS_DARK.read() {
        Ok(d) => *d,
        Err(err) => {
            error!("Poisoned {err:?}");
            false
        }
    }
}

#[macro_export]
macro_rules! format2 {
     ($template: expr $(,)?) => {
          let s = formatx::formatx!($template)
          s
     };

     ($template: expr, $($values: tt)*)  => {{
          let res = formatx::formatx!($template,$($values)*);
          match res {
               Ok(s) => s,
               Err(error) => {
                    let error_str = format!("Translation error: {:?}", error);
                    log::error!("{}",error_str);
                    error_str
               }
          }
     }};
}

#[macro_export]
macro_rules! upgrade {
    ($weak_ref:expr) => {
        upgrade!($weak_ref, ())
    };

    ($weak_ref:expr, $ret:expr) => {{
        let Some(weak_ref) = $weak_ref.upgrade() else {
            log::warn!("Reference upgrade failed {:?}", $weak_ref);
            return $ret;
        };
        weak_ref
    }};
}

#[macro_export]
macro_rules! upgrade_ret {
    ($weak_ref:expr, ret:expr) => {{
        let Some(weak_ref) = $weak_ref.upgrade() else {
            log::warn!("Reference upgrade failed {:?}", $weak_ref);
            return $ret;
        };
        weak_ref
    }};
}

#[macro_export]
macro_rules! upgrade_continue {
    ($weak_ref:expr) => {{
        let Some(weak_ref) = $weak_ref.upgrade() else {
            log::warn!("Reference upgrade failed {:?}", $weak_ref);
            continue;
        };
        weak_ref
    }};
}
