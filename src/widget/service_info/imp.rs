use gtk::{glib, prelude::*, subclass::prelude::*};
use log::error;

use crate::{
    systemd::{self, data::UnitInfo},
    systemd_gui,
    widget::info_window::InfoWindow,
};

#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/service_status.ui")]
pub struct ServiceStatusImp {
    #[template_child]
    pub name_description: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_loaded: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_dropin: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_active: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_mainpid: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_tasks: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_memory: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_cpu: TemplateChild<gtk::Label>,

    #[template_child]
    pub info_cgroup: TemplateChild<gtk::Label>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ServiceStatusImp {
    const NAME: &'static str = "ServiceStatus";
    type Type = super::ServiceStatus;
    type ParentType = gtk::Grid;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[gtk::template_callbacks]
impl ServiceStatusImp {
    #[template_callback(name = "handle_refresh_click")]
    fn handle_refresh_click(&self, _button: &gtk::Button) {
        systemd_gui::selected_unit(|unit: &UnitInfo| self.fill_data(unit));
    }

    #[template_callback]
    fn handle_all_details_click(_button: &gtk::Button) {
        systemd_gui::selected_unit(|unit: &UnitInfo| {
            let info_window = InfoWindow::new();

            info_window.fill_data(&unit);

            info_window.present();
        });
    }

    pub(super) fn fill_data(&self, unit: &UnitInfo) {
        self.name_description
            .set_label(&format!("{} - {}", unit.primary(), unit.description()));

        let map = match systemd::fetch_system_unit_info(&unit) {
            Ok(m) => m,
            Err(e) => {
                error!("Fail to retreive Unit info: {:?}", e);
                return;
            }
        };
        
        if let Some(load_state) = map.get("LoadState") {
            self.info_loaded
                .set_label(load_state);
        }

        if let Some(active_state) = map.get("ActiveState") {
            self.info_active
                .set_label(active_state);
        }

        if let Some(drop_in_paths) = map.get("DropInPaths") {
            self.info_dropin
                .set_label(drop_in_paths);
        }

        if let Some(main_pid) = map.get("MainPID") {
            self.info_mainpid
                .set_label(&format!("{main_pid} ({})", unit.display_name()));
        }
    }
}

impl ObjectImpl for ServiceStatusImp {}
impl WidgetImpl for ServiceStatusImp {}
impl GridImpl for ServiceStatusImp {}
