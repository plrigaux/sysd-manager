use log::{info, warn};

use crate::systemd::{self, data::UnitInfo, enums::EnablementStatus, errors::SystemdErrors};

use crate::gtk::prelude::*;

pub(super) fn switch_ablement_state_set(
    toast_overlay: &adw::ToastOverlay,
    expected_new_status: EnablementStatus,
    switch: &gtk::Switch,
    unit: &UnitInfo,
) {
    // handle_switch(&column_view, /*unit_ref,*/ enabled, switch);

    info!(
        "switch_ablement_state_set Unit \"{}\" enablement \"{}\" sw_active {} sw_state {} expected_new_status {expected_new_status}", unit.primary(), EnablementStatus::from(unit.enable_status()).as_str(),
        switch.is_active(),
        switch.state()
    );

    let current_enabled_status: EnablementStatus = unit.enable_status().into();

    if expected_new_status == current_enabled_status {
        set_switch_tooltip(current_enabled_status, switch, &unit.primary());
        return;
    }

    let enable_result = systemd::disenable_unit_file(unit, expected_new_status);

    match enable_result {
        Ok(enablement_status_ret) => {
            let toast_info = format!("{:?}", enablement_status_ret);
            info!("{toast_info}");

            let toast = adw::Toast::new(&toast_info);

            toast_overlay.add_toast(toast);

            unit.set_enable_status(expected_new_status as u32);

            let enabled_new = expected_new_status == EnablementStatus::Enabled;
            switch.set_state(enabled_new);
        }

        Err(error) => {
            let error_message = match error {
                SystemdErrors::SystemCtlError(s) => s,
                _ => format!("{:?}", error),
            };

            let toast_warn = format!(
                "Action \"{:?}\" on unit \"{}\" FAILED!\n{:?}",
                expected_new_status,
                unit.primary(),
                error_message
            );
            warn!("{toast_warn}");

            let toast = adw::Toast::new(&toast_warn);

            toast_overlay.add_toast(toast);
        }
    }

    //let unit_file_state =
    //    systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);
    //info!("New Status : {:?}", unit_file_state);

    handle_switch_sensivity(switch, unit, false);
}

fn set_switch_tooltip(enabled: EnablementStatus, switch: &gtk::Switch, unit_name: &str) {
    let enabled = enabled == EnablementStatus::Enabled;

    let action_text = if enabled { "Disable" } else { "Enable" };

    let text = format!("{action_text} unit <b>{unit_name}</b>");

    switch.set_tooltip_markup(Some(&text));
}

pub(super) fn handle_switch_sensivity(
    switch: &gtk::Switch,
    unit: &UnitInfo,
    check_current_state: bool,
) {
    let mut unit_file_state: EnablementStatus = unit.enable_status().into();

    if check_current_state {
        let current_state = match systemd::get_unit_file_state(unit) {
            Ok(a) => a,
            Err(_e) => {
                info!("Get unit state fail! For {:#?}.", unit.primary());
                unit_file_state
            }
        };

        if current_state != unit_file_state {
            unit_file_state = current_state;
            unit.set_enable_status(unit_file_state as u32);
        }
    }

    if unit_file_state == EnablementStatus::Enabled {
        switch.set_state(true);
        switch.set_active(true);
    } else {
        switch.set_state(false);
        switch.set_active(false);
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
