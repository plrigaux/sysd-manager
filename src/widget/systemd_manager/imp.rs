use std::{cell::RefCell, collections::HashMap};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    },
};
use tracing::{error, info, warn};

use crate::utils::palette::{green, red};

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/systemd_manager.ui")]
pub struct SystemdManagerDialogImp {
    #[template_child]
    sm_version_label: TemplateChild<gtk::Label>,
    #[template_child]
    sm_features_label: TemplateChild<gtk::Label>,
    #[template_child]
    sm_unit_path_label: TemplateChild<gtk::Label>,

    properties: RefCell<HashMap<String, zbus::OwnedVariant>>,
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

        let mut p = String::new();
        let mut first = true;
        for pp in s {
            p.push_str(&pp);
            if first {
                first = false;
            } else {
                p.push('\n');
            }
        }

        self.sm_unit_path_label.set_label(&p);
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
                match systemd::fetch_systemd_manager_properties(base::enums::UnitDBusLevel::System)
                {
                    Ok(map) => {
                        systemd_manager_window.imp().properties.replace(map);
                        systemd_manager_window.imp().fill_data();
                    }
                    Err(e) => error!("Fail to retreive Systemd Manager Properties: {e:?}"),
                }
            });
        });
    }
}

impl WidgetImpl for SystemdManagerDialogImp {}
impl WindowImpl for SystemdManagerDialogImp {}
impl AdwWindowImpl for SystemdManagerDialogImp {}
