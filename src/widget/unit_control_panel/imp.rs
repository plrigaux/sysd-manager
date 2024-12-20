use std::cell::{OnceCell, RefCell};

use adw::{subclass::prelude::*, Toast};
use gtk::{
    glib::{self, property::PropertySet},
    prelude::*,
};
use log::{debug, error, info, warn};

use crate::{
    systemd::{
        self,
        data::UnitInfo,
        enums::{ActiveState, StartStopMode},
    },
    widget::{
        journal::JournalPanel, kill_panel::KillPanel, unit_file_panel::UnitFilePanel,
        unit_info::UnitInfoPanel,
    },
};

use super::{controls, enums::UnitContolType, UnitControlPanel};
use strum::IntoEnumIterator;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_control_panel.ui")]
#[properties(wrapper_type = super::UnitControlPanel)]
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
    start_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    stop_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    kill_button: TemplateChild<gtk::Button>,

    #[template_child]
    restart_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    side_overlay: TemplateChild<adw::OverlaySplitView>,

    #[template_child]
    kill_panel: TemplateChild<KillPanel>,

    #[template_child]
    start_modes: TemplateChild<gtk::Box>,

    #[template_child]
    stop_modes: TemplateChild<gtk::Box>,

    #[template_child]
    restart_modes: TemplateChild<gtk::Box>,

    toast_overlay: OnceCell<adw::ToastOverlay>,

    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,

    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,
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

#[glib::derived_properties]
impl ObjectImpl for UnitControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_modes(&self.start_modes, UnitContolType::Start);
        self.set_modes(&self.stop_modes, UnitContolType::Stop);
        self.set_modes(&self.restart_modes, UnitContolType::Restart);
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
    fn button_start_clicked(&self, _button: &adw::SplitButton) {
        let unit = current_unit!(self);

        let mode: StartStopMode = (&self.start_mode).into();

        let start_results: Result<String, systemd::SystemdErrors> =
            systemd::start_unit(&unit, mode);

        self.start_restart(
            &unit,
            start_results,
            UnitContolType::Start,
            ActiveState::Active,
            mode,
        )
    }

    //Dry
    fn start_restart(
        &self,
        unit: &UnitInfo,
        start_results: Result<String, systemd::SystemdErrors>,
        action: UnitContolType,
        new_active_state: ActiveState,
        mode: StartStopMode,
    ) {
        let job_op = match start_results {
            Ok(job) => {
                let info = format!(
                    "Unit \"{}\" has been {}ed with mode {:?}!",
                    unit.primary(),
                    action.as_str(),
                    mode.as_str()
                );
                info!("{info}");

                let toast = Toast::new(&info);
                self.toast_overlay.get().unwrap().add_toast(toast);

                controls::update_active_state(unit, new_active_state);

                Some(job)
            }
            Err(e) => {
                error!(
                    "Can't {} the unit {:?}, because: {:?}",
                    action.as_str(),
                    unit.primary(),
                    e
                );
                None
            }
        };

        let Some(_job) = job_op else {
            return;
        };

        if unit.pathexists() {
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
    fn button_stop_clicked(&self, _button: &adw::SplitButton) {
        let unit = current_unit!(self);
        let mode: StartStopMode = (&self.stop_mode).into();
        let stop_results = systemd::stop_unit(&unit, mode);
        self.start_restart(
            &unit,
            stop_results,
            UnitContolType::Stop,
            ActiveState::Inactive,
            mode,
        )
    }

    #[template_callback]
    fn button_restart_clicked(&self, _button: &adw::SplitButton) {
        let unit = current_unit!(self);
        let mode: StartStopMode = (&self.restart_mode).into();
        let start_results = systemd::restart_unit(&unit, mode);
        self.start_restart(
            &unit,
            start_results,
            UnitContolType::Restart,
            ActiveState::Active,
            mode,
        )
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

    fn set_modes(&self, modes_box: &gtk::Box, control_type: UnitContolType) {
        let default = StartStopMode::Fail;
        let mut ck_group: Option<gtk::CheckButton> = None;

        for mode in StartStopMode::iter() {
            if control_type == UnitContolType::Stop && mode == StartStopMode::Isolate {
                continue;
            }

            let ck = gtk::CheckButton::builder().label(mode.as_str()).build();

            modes_box.append(&ck);

            let source_property = format!("{}_mode", control_type.as_str());
            let unit_control_panel = self.obj();
            ck.bind_property(
                "active",
                &unit_control_panel as &UnitControlPanel,
                &source_property,
            )
            .transform_to(move |_, _active: bool| Some(mode.as_str()))
            .build();

            if mode == default {
                ck.set_active(true);
            }

            if ck_group.is_none() {
                ck_group = Some(ck);
            } else {
                ck.set_group(ck_group.as_ref());
            }
        }
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
