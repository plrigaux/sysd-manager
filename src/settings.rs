use crate::systemd;
use log::info;
use log::warn;

pub fn set_color_scheme(settings: &gtk::Settings) {
    let mut cmd = systemd::commander(&[
        "gsettings",
        "get",
        "org.gnome.desktop.interface",
        "color-scheme",
    ]);

    let out = match cmd.output() {
        Ok(v) => v.stdout,
        Err(e) => {
            warn!(
                "Can't get gsettings org.gnome.desktop.interface color-scheme. Error: {:?}",
                e
            );
            return;
        }
    };

    let color_scheme_value = match String::from_utf8(out) {
        Ok(value) => value.trim().to_owned(),
        Err(e) => {
            warn!("Error parsing color-scheme: {:?}", e);
            return;
        }
    };

    info!("color-scheme value: \"{}\"", color_scheme_value);

    if color_scheme_value.contains("prefer-dark") {
        set_application_prefer_dark_theme(settings, true);
    } else if color_scheme_value.contains("default") {
        set_application_prefer_dark_theme(settings, false);
    }
}

fn set_application_prefer_dark_theme(settings: &gtk::Settings, prefer_dark_theme: bool) {
    info!("set_gtk_application_prefer_dark_theme {prefer_dark_theme}");
    settings.set_gtk_application_prefer_dark_theme(prefer_dark_theme);
}
