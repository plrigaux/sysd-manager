use crate::gtk::prelude::SettingsExt;
use gtk::gio;
use log::info;
use log::warn;

pub fn set_color_scheme(settings: &gtk::Settings) {
    const SCHEMA: &str = "org.gnome.desktop.interface";
    const KEY: &str = "color-scheme";

    let Some(settings_schema_source) = gio::SettingsSchemaSource::default() else {
        warn!("Can't intanciate SettingsSchemaSource");
        return;
    };

    let Some(setting_schema) = settings_schema_source.lookup(SCHEMA, true) else {
        warn!("Schema '{}' not found", SCHEMA);
        return;
    };

    if !setting_schema.has_key(KEY) {
        warn!("Key '{}' not found on schema '{}'", KEY, SCHEMA);
    }

    let gio_settings = gio::Settings::new(SCHEMA);

    let color_scheme_value = gio_settings.value(KEY);

    let Some(color_scheme) = color_scheme_value.str() else {
        warn!("Key '{}' not a string", KEY);
        return;
    };

    /*     let mut cmd = systemd::commander(&[
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
    }; */

    set_dark_theme(color_scheme, settings);
}

fn set_dark_theme(color_scheme: &str, settings: &gtk::Settings) {
    info!("color-scheme value: '{}'", color_scheme);

    if color_scheme.contains("prefer-dark") {
        set_application_prefer_dark_theme(settings, true);
    } else if color_scheme.contains("default") {
        set_application_prefer_dark_theme(settings, false);
    }
}

fn set_application_prefer_dark_theme(settings: &gtk::Settings, prefer_dark_theme: bool) {
    info!("set_gtk_application_prefer_dark_theme {prefer_dark_theme}");
    settings.set_gtk_application_prefer_dark_theme(prefer_dark_theme);
}
