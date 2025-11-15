use gtk::gio::Settings;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

pub fn new_settings() -> Settings {
    Settings::new(APP_ID)
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
