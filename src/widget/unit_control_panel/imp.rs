use std::cell::{OnceCell, RefCell};

use adw::{subclass::prelude::*, Toast};
use gtk::{
    glib::{self, property::PropertySet},
    prelude::*,
};
use log::{debug, error, info, warn};

use crate::{
    systemd::{self, data::UnitInfo, enums::ActiveState},
    widget::{
        journal::JournalPanel, kill_panel::KillPanel, unit_file_panel::UnitFilePanel,
        unit_info::UnitInfoPanel,
    },
};

use super::controls;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_control_panel.ui")]
pub struct UnitControlPanelImpl {
    #[template_child]
    unit_info_panel: TemplateChild<UnitInfoPanel>,

    #[template_child]
    unit_file_panel: TemplateChild<UnitFilePanel>,

    #[template_child]
    unit_journal_panel: TemplateChild<JournalPanel>,

    #[template_child]
    ablement_switch: TemplateChild<gtk::Switch>,

    #[template_child]
    start_button: TemplateChild<gtk::Button>,

    #[template_child]
    stop_button: TemplateChild<gtk::Button>,

    #[template_child]
    kill_button: TemplateChild<gtk::Button>,

    #[template_child]
    restart_button: TemplateChild<gtk::Button>,

    #[template_child]
    side_overlay: TemplateChild<adw::OverlaySplitView>,

    #[template_child]
    kill_panel: TemplateChild<KillPanel>,

    toast_overlay: OnceCell<adw::ToastOverlay>,

    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,
}

#[glib::object_subclass]
impl ObjectSubclass for UnitControlPanelImpl {
    const NAME: &'static str = "UnitControlPanel";
    type Type = super::UnitControlPanel;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

macro_rules! current_unit {
    ($app:expr) => {{
        current_unit!($app, ())
    }};

    ($app:expr, $opt:expr) => {{
        let unit_op = $app.current_unit.borrow();
        let Some(unit) = unit_op.as_ref() else {
            warn!("No selected unit!");
            return $opt;
        };

        unit.clone()
    }};
}

impl ObjectImpl for UnitControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

#[gtk::template_callbacks]
impl UnitControlPanelImpl {
    pub(super) fn set_overlay(&self, toast_overlay: &adw::ToastOverlay) {
        self.kill_panel.register(&self.side_overlay, toast_overlay);

        if let Err(e) = self.toast_overlay.set(toast_overlay.clone()) {
            warn!("Set Toast Overlay Issue: {:?}", e)
        }
    }

    #[template_callback]
    fn switch_ablement_state_set(&self, switch_new_state: bool, switch: &gtk::Switch) -> bool {
        info!(
            "switch_ablement_state_set new {switch_new_state} old {}",
            switch.state()
        );

        if switch_new_state == switch.state() {
            debug!("no state change");
            return true;
        }

        let unit = current_unit!(self, true);

        controls::switch_ablement_state_set(
            &self.toast_overlay.get().unwrap(),
            switch_new_state,
            switch,
            &unit,
        );

        self.unit_info_panel.display_unit_info(&unit);
        true // to stop the signal emission
    }

    #[template_callback]
    fn button_start_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        let start_results: Result<String, systemd::SystemdErrors> = systemd::start_unit(&unit);

        self.start_restart(&unit, start_results, "start", ActiveState::Active)
    }

    //Dry
    fn start_restart(
        &self,
        unit: &UnitInfo,
        start_results: Result<String, systemd::SystemdErrors>,
        action: &str,
        new_active_state: ActiveState,
    ) {
        let job_op = match start_results {
            Ok(job) => {
                let info = format!("Unit \"{}\" has been {action}ed!", unit.primary());
                info!("{info}");

                let toast = Toast::new(&info);
                self.toast_overlay.get().unwrap().add_toast(toast);

                controls::update_active_state(unit, new_active_state);

                Some(job)
            }
            Err(e) => {
                error!(
                    "Can't {action} the unit {:?}, because: {:?}",
                    unit.primary(),
                    e
                );
                None
            }
        };

        let Some(_job) = job_op else {
            return;
        };

        if unit.pathexist() {
            self.unit_info_panel.display_unit_info(&unit);
            return;
        }

        match systemd::get_unit_object_path(&unit) {
            Ok(object_path) => {
                unit.set_object_path(object_path);
                self.unit_info_panel.display_unit_info(&unit);
            }
            Err(e) => warn!(
                "Can't retrieve unit's {:?} object path, because: {:?}",
                unit.primary(),
                e
            ),
        }
    }

    #[template_callback]
    fn button_stop_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        let stop_results = systemd::stop_unit(&unit);
        self.start_restart(&unit, stop_results, "stop", ActiveState::Inactive)
    }

    #[template_callback]
    fn button_restart_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        let start_results = systemd::restart_unit(&unit);
        self.start_restart(&unit, start_results, "restart", ActiveState::Active)
    }

    #[template_callback]
    fn button_kill_clicked(&self, _button: &gtk::Button) {
        let unit = current_unit!(self);

        self.kill_panel.set_unit(&unit);

        let collapsed = self.side_overlay.is_collapsed();
        self.side_overlay.set_collapsed(!collapsed);
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.search_bar
            .borrow()
            .set_search_mode(toggle_button.is_active());
    }

    pub(super) fn selection_change(&self, unit: &UnitInfo) {
        self.current_unit.set(Some(unit.clone()));

        self.unit_info_panel.display_unit_info(unit);
        self.unit_file_panel.set_file_content(unit);
        self.unit_journal_panel.display_journal(unit);
        self.kill_panel.set_unit(unit);

        controls::handle_switch_sensivity(&self.ablement_switch, unit, true);

        self.start_button.set_sensitive(true);
        self.stop_button.set_sensitive(true);
        self.restart_button.set_sensitive(true);
        self.kill_button.set_sensitive(true);
    }

    pub(super) fn set_dark(&self, is_dark: bool) {
        self.unit_file_panel.set_dark(is_dark);
        self.unit_info_panel.set_dark(is_dark);
        self.unit_journal_panel.set_dark(is_dark);
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
