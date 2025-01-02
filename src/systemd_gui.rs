use gtk::gio::Settings;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

pub fn new_settings() -> Settings {
     Settings::new(APP_ID)
}
