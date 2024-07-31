use crate::gtk::prelude::*;
use crate::settings;
use crate::systemd_gui;
use gtk::gio;
use gtk::gio::Settings;
use gtk::glib;

use crate::gtk::subclass::prelude::*;
use lazy_static::lazy_static;
use log::info;
use log::warn;
lazy_static! {
    static ref EXAMPLE: Preferences = Preferences::new();
}

const KEY_DBUS_LEVEL: &str = "dbus-level";

pub fn build_preferences() -> gtk::Window {
    let settings = get_settings();

    let gbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);

    gbox.append(&gtk::Label::new(Some("DBus level")));
    gbox.set_vexpand(false);

    let model = gtk::StringList::new(&[DbusLevel::System.as_str(), DbusLevel::Session.as_str()]);
    let tb_system = gtk::DropDown::new(Some(model), gtk::Expression::NONE);
    tb_system.set_vexpand(false);

    tb_system.connect_selected_notify(|toggle_button| {
        let idx = toggle_button.selected();
        println!("Values Selecte {:?}", toggle_button.selected());

        let level: DbusLevel = idx.into();

        let settings = get_settings();
        if let Err(e) = settings.set_string(KEY_DBUS_LEVEL, level.as_str()) {
            warn!("Error: {:?}", e);
            return;
        }
        info!(
            "Save setting '{KEY_DBUS_LEVEL}' with value '{:?}'",
            level.as_str()
        )
    });

    gbox.append(&tb_system);
    //gbox.append(&tb_session);

    match setup_settings(settings) {
        DbusLevel::Session => tb_system.set_selected(1),
        DbusLevel::System => tb_system.set_selected(0),
    }

    let window = gtk::Window::builder()
        .title("Preferences")
        .default_height(600)
        .default_width(600)
        .child(&gbox)
        .build();

    window
}

fn get_settings() -> Settings {
    gio::Settings::new(systemd_gui::APP_ID)
}

fn setup_settings(settings: Settings) -> DbusLevel {
    let level: glib::GString = settings.string(KEY_DBUS_LEVEL);
    DbusLevel::from(level.as_str())
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DbusLevel {
    Session,
    System,
}

impl DbusLevel {
    fn as_str(&self) -> &str {
        match self {
            DbusLevel::Session => "Session",
            DbusLevel::System => "System",
        }
    }
}

impl From<&str> for DbusLevel {
    fn from(level: &str) -> Self {
        if "System".eq(level) {
            DbusLevel::System
        } else {
            DbusLevel::Session
        }
    }
}

impl From<u32> for DbusLevel {
    fn from(level: u32) -> Self {
        if level == 0 {
            DbusLevel::System
        } else {
            DbusLevel::Session
        }
    }
}

glib::wrapper! {
    pub struct Preferences(ObjectSubclass<imp::PreferencesImp>);
}

impl Preferences {
    pub fn new() -> Self {
        let this_object: Self = glib::Object::new();
        let imp: &imp::PreferencesImp = this_object.imp();

        let settings = get_settings();
        let val = setup_settings(settings);
        imp.set_dbus_level(val.as_str().to_string());

        this_object
    }
}

pub mod imp {
    use std::sync::Mutex;

    use gtk::{glib, prelude::*, subclass::prelude::*};
    use log::warn;

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::Preferences)]
    pub struct PreferencesImp {
        #[property(get, set = Self::set_dbus_level )]
        pub(super) dbus_level: Mutex<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesImp {
        const NAME: &'static str = "PreferencesImp";
        type Type = super::Preferences;

        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PreferencesImp {}

    impl PreferencesImp {
        pub fn set_dbus_level( & self, dbus_level: String) {
            match self.dbus_level.lock() {
                Ok(mut a) => *a = dbus_level,
                Err(e) => warn!("Error {:?}", e),
            }
        }
    }
}
