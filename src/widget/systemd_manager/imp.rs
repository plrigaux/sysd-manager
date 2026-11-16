use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use base::enums::UnitDBusLevel;
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    },
};
use systemd::data::UnitInfo;
use tracing::{error, info, warn};

use crate::{
    systemd_gui::is_dark,
    upgrade,
    utils::palette::{green, red},
    widget::{InterPanelMessage, unit_file_panel::UnitFilePanel},
};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/systemd_manager.ui")]
pub struct SystemdManagerDialogImp {
    #[template_child]
    sm_version_label: TemplateChild<gtk::Label>,
    #[template_child]
    sm_features_label: TemplateChild<gtk::Label>,
    #[template_child]
    sm_unit_path_label: TemplateChild<gtk::Label>,
    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,
    #[template_child]
    sm_file_panel: TemplateChild<UnitFilePanel>,
    #[template_child]
    systemd_manager_stack: TemplateChild<adw::ViewStack>,

    properties: RefCell<HashMap<String, zbus::OwnedVariant>>,
    level: Cell<UnitDBusLevel>,
}

impl SystemdManagerDialogImp {
    fn fill_data(&self) {
        let map = self.properties.borrow();

        let s = get_var(&map, "Version");

        self.sm_version_label.set_label(s);

        let s = get_var(&map, "Features");

        let red = red().get_color();
        let green = green().get_color();
        let mut color_features = String::new();

        for feature in s.split(' ') {
            let color = if feature.starts_with('+') { green } else { red };

            color_features.push_str("<span fgcolor='");
            color_features.push_str(color);
            color_features.push_str("'>");
            color_features.push_str(feature);
            color_features.push_str("</span> ");
        }

        self.sm_features_label.set_label(&color_features);
        let mut s = get_var2(&map, "UnitPath");

        s.sort();

        let mut path_label_text = String::new();
        let mut first = true;
        for path in s {
            path_label_text.push_str(&path);
            if first {
                first = false;
            } else {
                path_label_text.push('\n');
            }
        }

        self.sm_unit_path_label.set_label(&path_label_text);
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

        self.sm_file_panel
            .set_inter_message(&InterPanelMessage::UnitChange(Some(&unit)));

        self.sm_file_panel
            .set_inter_message(&InterPanelMessage::IsDark(is_dark()));
    }
}

fn get_var<'a>(map: &'a HashMap<String, zbus::OwnedVariant>, key: &'a str) -> &'a str {
    map.get(key)
        .or_else(|| {
            warn!("key {key} not found!");
            None
        })
        .and_then(|v| {
            v.downcast_ref::<&str>()
                .inspect_err(|e| warn!("Key {key}, variant downcast Failed: {:?}", e))
                .ok()
        })
        .unwrap_or_default()
}

fn get_var2<'a>(map: &'a HashMap<String, zbus::OwnedVariant>, key: &'a str) -> Vec<String> {
    let Some(v) = map.get(key) else {
        warn!("key {key} not found!");
        return vec![];
    };

    <zbus::Variant<'_> as Clone>::clone(v)
        .downcast::<Vec<String>>()
        .inspect_err(|e| warn!("Key {key}, variant downcast Failed: {:?}", e))
        .unwrap_or_default()
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

        self.obj().connect_show(|systemd_manager_window| {
            info!("Show Systemd Manager Dialog");
            let systemd_manager_window = systemd_manager_window.clone();
            glib::spawn_future_local(async move {
                match systemd::fetch_systemd_manager_properties(
                    systemd_manager_window.imp().level.get(),
                ) {
                    Ok(map) => {
                        systemd_manager_window.imp().properties.replace(map);
                        systemd_manager_window.imp().fill_data();
                    }
                    Err(e) => error!("Fail to retreive Systemd Manager Properties: {e:?}"),
                }
            });
        });

        let dialog_weak_ref = self.downgrade();
        self.systemd_manager_stack
            .connect_visible_child_name_notify(move |view_stack| {
                let dialog = upgrade!(dialog_weak_ref);
                if view_stack.visible_child_name().as_deref() == Some("file") {
                    dialog
                        .sm_file_panel
                        .set_inter_message(&InterPanelMessage::PanelVisible(true));
                }
            });
    }
}

impl WidgetImpl for SystemdManagerDialogImp {}
impl WindowImpl for SystemdManagerDialogImp {}
impl AdwWindowImpl for SystemdManagerDialogImp {}
