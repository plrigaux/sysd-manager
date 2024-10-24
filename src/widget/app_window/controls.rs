use adw::Toast;
use log::{debug, info, warn};

use crate::systemd::{self, data::UnitInfo, enums::EnablementStatus};

use super::imp::AppWindowImpl;
use crate::gtk::prelude::*;

pub(super) fn switch_ablement_state_set(
    app_win: &AppWindowImpl,
    state: bool,
    switch: &gtk::Switch,
    unit: &UnitInfo,
) {
    // handle_switch(&column_view, /*unit_ref,*/ enabled, switch);

    debug!(
        "active {} state {} new {state}",
        switch.is_active(),
        switch.state()
    );

    let enabled_status: EnablementStatus = unit.enable_status().into();

    if state && enabled_status == EnablementStatus::Enabled
        || !state && enabled_status != EnablementStatus::Enabled
    {
        set_switch_tooltip(enabled_status, switch, &unit.primary());
        return;
    }

    let (enable_result, action) = if state {
        (systemd::enable_unit_files(&unit), EnablementStatus::Enabled)
    } else {
        (
            systemd::disable_unit_files(&unit),
            EnablementStatus::Disabled,
        )
    };

    match enable_result {
        Ok(enablement_status_ret) => {
            let toast_info = format!(
                "New active statut ({}) for unit {}",
                enablement_status_ret.to_string(),
                unit.primary(),
            );
            info!("{toast_info}");

            let toast = Toast::new(&toast_info);

            app_win.toast_overlay.add_toast(toast);
        }

        Err(error) => {
            let error_message = match error {
                systemd::SystemdErrors::SystemCtlError(s) => s,
                _ => format!("{:?}", error),
            };
            let toast_warn = format!(
                "Action \"{:?}\" on unit \"{}\": FAILED! {:?}",
                action,
                unit.primary(),
                error_message
            );
            warn!("{toast_warn}");

            let toast = Toast::new(&toast_warn);

            app_win.toast_overlay.add_toast(toast);

            //TODO put a timer to set back the switch
        }
    }

    //let unit_file_state =
    //    systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);
    //info!("New Status : {:?}", unit_file_state);

    let enabled_new = action == EnablementStatus::Enabled;
    switch.set_state(enabled_new);

    unit.set_enable_status(action.to_string());

    handle_switch_sensivity(action, switch, unit);
}

fn set_switch_tooltip(enabled: EnablementStatus, switch: &gtk::Switch, unit_name: &str) {
    let enabled = enabled == EnablementStatus::Enabled;

    let action_text = if enabled { "Disable" } else { "Enable" };

    let text = format!("{action_text} unit <b>{unit_name}</b>");

    switch.set_tooltip_markup(Some(&text));
}

pub(super) fn handle_switch_sensivity(
    mut unit_file_state: EnablementStatus,
    switch: &gtk::Switch,
    unit: &UnitInfo,
) {
    if unit_file_state == EnablementStatus::Unknown {
        unit_file_state = unit.enable_status().into();

        if unit_file_state == EnablementStatus::Enabled {
            switch.set_state(true);
            switch.set_active(true);
        } else {
            switch.set_state(false);
            switch.set_active(false);
        }
    }

    let sensitive = if unit_file_state == EnablementStatus::Enabled
        || unit_file_state == EnablementStatus::Disabled
    {
        set_switch_tooltip(unit_file_state, switch, &unit.primary());

        true
    } else {
        switch.set_tooltip_text(None);
        false
    };

    switch.set_sensitive(sensitive);
}
