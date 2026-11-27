use crate::{
    systemd_gui::new_settings,
    upgrade,
    utils::{
        palette::{green, red},
        ztool::{get_array_str, value_to},
    },
    widget::close_window_shortcut,
};
use adw::{prelude::*, subclass::window::AdwWindowImpl};
use base::enums::UnitDBusLevel;
use gettextrs::pgettext;
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    },
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
};
use systemd::data::UnitInfo;
use tokio::fs;
use tracing::{error, info, warn};

const LOG_LEVELS: &[&str; 8] = &[
    "emerg", "alert", "crit", "err", "warning", "notice", "info", "debug",
];

const LOG_TARGETS: &[&str; 5] = &[
    "console",
    "console-prefixed",
    "kmsg",
    "journal-or-kmsg",
    "auto",
];

const WINDOW_SIZE: &str = "systemd-manager-window-size";
#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/systemd_manager.ui")]
pub struct SystemdManagerDialogImp {
    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,
    #[template_child]
    systemd_manager_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    file_container: TemplateChild<gtk::Box>,
    #[template_child]
    param_grid: TemplateChild<gtk::Grid>,
    #[template_child]
    log_level_combo_row: TemplateChild<adw::ComboRow>,
    #[template_child]
    log_target_combo_row: TemplateChild<adw::ComboRow>,

    properties: RefCell<HashMap<String, zbus::OwnedVariant>>,
    level: Cell<UnitDBusLevel>,
}

impl SystemdManagerDialogImp {
    fn set_config_label(&self, label: &str, row: i32) {
        let label = gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .yalign(0.0)
            .build();
        self.param_grid.attach(&label, 0, row, 1, 1);
    }

    fn set_config_value(&self, label: &str, row: i32) {
        let label = gtk::Label::builder()
            .label(label)
            .xalign(0.0)
            .use_markup(true)
            .wrap(true)
            .selectable(true)
            .build();
        self.param_grid.attach(&label, 1, row, 1, 1);
    }

    fn fill_data(&self) {
        let map = self.properties.borrow();

        let mut row = 0;
        let param_label = pgettext("manager", "Version");
        self.set_config_label(&param_label, row);

        let data_label = get_var(&map, "Version");
        self.set_config_value(data_label, row);

        row += 1;

        let param_label = pgettext("manager", "Architecture");
        self.set_config_label(&param_label, row);
        let data_label = get_var(&map, "Architecture");
        self.set_config_value(data_label, row);

        let features = get_var(&map, "Features");

        let red = red().get_color();
        let green = green().get_color();
        let mut color_features = String::new();

        for feature in features.split(' ') {
            let color = if feature.starts_with('+') { green } else { red };

            color_features.push_str("<span fgcolor='");
            color_features.push_str(color);
            color_features.push_str("'>");
            color_features.push_str(feature);
            color_features.push_str("</span> ");
        }

        row += 1;
        let param_label = pgettext("manager", "Features");
        self.set_config_label(&param_label, row);

        self.set_config_value(&color_features, row);

        row += 1;
        let param_label = pgettext("manager", "Log Level");
        self.set_config_label(&param_label, row);

        let log_level = get_var(&map, "LogLevel");
        self.set_config_value(log_level, row);

        row += 1;
        let param_label = pgettext("manager", "Log Target");
        self.set_config_label(&param_label, row);

        let log_level = get_var(&map, "LogTarget");
        self.set_config_value(log_level, row);

        row += 1;
        let param_label = pgettext("manager", "Environment Variables");
        self.set_config_label(&param_label, row);

        let env_vars = get_vars(&map, "Environment");
        let env_vars = str_array_to_string(&env_vars);
        self.set_config_value(&env_vars, row);

        row += 1;
        let param_label = pgettext("manager", "Unit Paths");
        self.set_config_label(&param_label, row);

        let path_label_text = get_path(map);

        self.set_config_value(&path_label_text, row);
    }

    pub(crate) fn set_level(&self, level: base::enums::UnitDBusLevel) {
        self.level.set(level);
        self.window_title.set_subtitle(&level.nice_label());

        let unit = UnitInfo::from_unit_key("systemd.manager", level);

        let file_path = match level {
            UnitDBusLevel::UserSession => "/etc/systemd/user.conf",
            _ => "/etc/systemd/system.conf",
        };

        unit.set_file_path(Some(file_path));
    }

    pub fn save_window_context(&self) -> Result<(), glib::BoolError> {
        let size = self.obj().default_size().to_variant();

        let settings = new_settings();

        settings.set_value(WINDOW_SIZE, &size)?;

        Ok(())
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = new_settings();

        let size = settings.value(WINDOW_SIZE);

        let (width, height) = size.get::<(i32, i32)>().unwrap();

        // Set the size of the window
        self.obj().set_default_size(width, height);
    }

    fn write_file_section(&self, title_label: &str, files: Vec<String>) {
        self.write_file_section_sub(title_label, files, "title-2")
    }

    fn write_file_section_sub(&self, title_label: &str, files: Vec<String>, section_style: &str) {
        if !files.is_empty() {
            let link = gtk::Label::builder()
                .label(title_label)
                .halign(gtk::Align::Start)
                .css_classes([section_style])
                .build();
            self.file_container.append(&link);
        }

        for preset_file in files {
            let link = gtk::LinkButton::builder()
                .uri(format!("file://{}", preset_file))
                .label(preset_file)
                .halign(gtk::Align::Start)
                .build();
            self.file_container.append(&link);
        }
    }
}

fn get_path(map: std::cell::Ref<'_, HashMap<String, zbus::OwnedVariant>>) -> String {
    let mut paths = get_vars(&map, "UnitPath");

    paths.sort();

    str_array_to_string(&paths)
}

fn str_array_to_string(values: &[&str]) -> String {
    values.join("\n")
}

fn get_var<'a>(map: &'a HashMap<String, zbus::OwnedVariant>, key: &'a str) -> &'a str {
    map.get(key)
        .or_else(|| {
            warn!("key {key} not found!");
            None
        })
        .map(|v| value_to::<&str>(v))
        .unwrap_or_default()
}

fn get_vars<'a>(map: &'a HashMap<String, zbus::OwnedVariant>, key: &'a str) -> Vec<&'a str> {
    let Some(v) = map.get(key) else {
        warn!("key {key} not found!");
        return vec![];
    };

    get_array_str(v)
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SystemdManagerDialogImp {
    const NAME: &'static str = "SystemdManagerDialog";
    type Type = super::SystemdManagerDialog;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SystemdManagerDialogImp {
    fn constructed(&self) {
        self.parent_constructed();
        info!("New Systemd Manager");
        close_window_shortcut(self.obj().as_ref());
        self.load_window_size();

        self.obj().connect_show(|systemd_manager_window| {
            info!("Show Systemd Manager Dialog");
            let systemd_manager_window = systemd_manager_window.clone();
            glib::spawn_future_local(async move {
                match systemd::fetch_systemd_manager_properties(
                    systemd_manager_window.imp().level.get(),
                ) {
                    Ok(map) => {
                        {
                            let log_level = get_var(&map, "LogLevel");

                            if let Some((index, _)) = LOG_LEVELS
                                .iter()
                                .enumerate()
                                .find(|(_, lv)| **lv == log_level)
                            {
                                systemd_manager_window
                                    .imp()
                                    .log_level_combo_row
                                    .set_selected(index as u32);
                            } else {
                                warn!("Log Level in not found for value {:?}", log_level);
                                systemd_manager_window
                                    .imp()
                                    .log_level_combo_row
                                    .set_sensitive(false);
                            }

                            let log_target = get_var(&map, "LogTarget");

                            if let Some((index, _)) = LOG_TARGETS
                                .iter()
                                .enumerate()
                                .find(|(_, lv)| **lv == log_target)
                            {
                                systemd_manager_window
                                    .imp()
                                    .log_target_combo_row
                                    .set_selected(index as u32);
                            } else {
                                warn!("Log Target not found for value {:?}", log_target);
                                systemd_manager_window
                                    .imp()
                                    .log_target_combo_row
                                    .set_sensitive(false);
                            }
                        }
                        systemd_manager_window.imp().properties.replace(map);
                        systemd_manager_window.imp().fill_data();
                    }
                    Err(e) => error!("Fail to retreive Systemd Manager Properties: {e:?}"),
                }
            });
        });

        let dialog_weak_ref = self.downgrade();
        glib::spawn_future_local(async move {
            let dialog = upgrade!(dialog_weak_ref);

            let (system_files, user_files) = systemd::runtime().block_on(get_conf());
            let (system_dropins, user_dropins) = systemd::runtime().block_on(get_dropins());

            let title_label = pgettext("manager", "Configuration System");
            dialog.write_file_section(&title_label, system_files);

            let title_label = pgettext("manager", "Drop-ins");
            dialog.write_file_section_sub(&title_label, system_dropins, "title-4");

            let title_label = pgettext("manager", "Configuration User");
            dialog.write_file_section(&title_label, user_files);

            let title_label = pgettext("manager", "Drop-ins");
            dialog.write_file_section_sub(&title_label, user_dropins, "title-4");

            let dbus_policies = systemd::runtime().block_on(get_dbus_policies());
            let title_label = pgettext("manager", "D-Bus Policies");
            dialog.write_file_section(&title_label, dbus_policies);
        });

        let dialog_weak_ref = self.downgrade();
        glib::spawn_future_local(async move {
            let dialog = upgrade!(dialog_weak_ref);
            let preset_files = systemd::runtime().block_on(get_preset());

            let title_label = pgettext("manager", "Preset");
            dialog.write_file_section(&title_label, preset_files);
        });

        let string_list = gtk::StringList::new(LOG_LEVELS);
        self.log_level_combo_row.set_model(Some(&string_list));
        let string_list = gtk::StringList::new(LOG_TARGETS);
        self.log_target_combo_row.set_model(Some(&string_list));
    }
}

async fn get_preset() -> Vec<String> {
    let preset_dir = [
        "/etc/systemd/system-preset",
        "/run/systemd/system-preset",
        "/usr/local/lib/systemd/system-preset",
        "/usr/lib/systemd/system-preset",
        "/etc/systemd/user-preset",
        "/run/systemd/user-preset",
        "/usr/local/lib/systemd/user-preset",
        "/usr/lib/systemd/user-preset",
        "/etc/systemd/initrd-preset",
        "/run/systemd/initrd-preset",
        "/usr/local/lib/systemd/initrd-preset",
        "/usr/lib/systemd/initrd-preset",
    ];
    let mut files = Vec::new();

    for dir in preset_dir {
        if let Err(e) = find_files(dir, "preset", &mut files).await {
            warn!("find preset file error {:?}", e);
        }
    }
    files
}

async fn find_files<T>(dir: T, extension: &str, results: &mut Vec<String>) -> std::io::Result<()>
where
    PathBuf: From<T>,
{
    let dir = PathBuf::from(dir);

    if !dir.is_dir() {
        return Ok(());
    }

    let mut entries = fs::read_dir(dir).await?;

    let mut dir_files = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file()
            && let Some(ext) = path.extension()
            && ext == extension
        {
            dir_files.push(path.to_string_lossy().to_string());
        }
    }

    dir_files.sort();

    results.append(&mut dir_files);
    Ok(())
}

async fn get_conf() -> (Vec<String>, Vec<String>) {
    let conf_dir = [
        "/etc/systemd/",
        "/run/systemd/",
        "/usr/local/lib/systemd/",
        "/usr/lib/systemd/",
    ];

    let mut system_files = Vec::new();
    let mut user_files = Vec::new();

    if let Some(home) = std::env::home_dir() {
        let file = home.join(".config/systemd/user.conf");

        if file.exists() {
            user_files.push(file.to_string_lossy().to_string());
        }
    }

    for dir in conf_dir {
        let dir = PathBuf::from(dir);

        if !dir.is_dir() {
            continue;
        }

        let file = dir.join("system.conf");

        if file.exists() {
            system_files.push(file.to_string_lossy().to_string());
        }

        let file = dir.join("user.conf");

        if file.exists() {
            user_files.push(file.to_string_lossy().to_string());
        }
    }

    (system_files, user_files)
}

async fn get_dropins() -> (Vec<String>, Vec<String>) {
    let conf_dir = [
        "/etc/systemd/",
        "/run/systemd/",
        "/usr/local/lib/systemd/",
        "/usr/lib/systemd/",
    ];

    let mut system_dropins = Vec::new();
    let mut user_dropins = Vec::new();

    for dir in conf_dir {
        let dir = PathBuf::from(dir);

        if !dir.is_dir() {
            continue;
        }

        let d_dir = dir.join("system.conf.d");

        if let Err(e) = find_files(d_dir, "conf", &mut system_dropins).await {
            warn!("find dropin file error {:?}", e);
        }

        let d_dir = dir.join("user.conf.d");

        if let Err(e) = find_files(d_dir, "conf", &mut user_dropins).await {
            warn!("find dropin file error {:?}", e);
        }
    }
    (system_dropins, user_dropins)
}

async fn get_dbus_policies() -> Vec<String> {
    let conf_dir = [
        "/usr/share/dbus-1/system.d/org.freedesktop.systemd1.conf",
        "/etc/dbus-1/system.d/org.freedesktop.systemd1.conf",
    ];

    let mut system_dropins = Vec::new();

    for file in conf_dir {
        let filep = PathBuf::from(file);

        if filep.is_file() {
            system_dropins.push(file.to_string());
        }
    }
    system_dropins
}

impl WidgetImpl for SystemdManagerDialogImp {}
impl WindowImpl for SystemdManagerDialogImp {
    fn close_request(&self) -> glib::Propagation {
        if let Err(err) = self.save_window_context() {
            error!("Failed to save window state {:?}", err);
        }

        self.parent_close_request();
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for SystemdManagerDialogImp {}
