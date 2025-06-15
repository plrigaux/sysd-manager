use gettextrs::pgettext;
use gtk::{gio, glib};
use log::{debug, info, warn};

use crate::format2;
use crate::systemd::{self, data::UnitInfo, enums::EnablementStatus, errors::SystemdErrors};

use super::UnitControlPanel;
use crate::gtk::prelude::*;
use crate::utils::palette::blue;

pub(super) fn switch_ablement_state_set(
    control_panel: &UnitControlPanel,
    expected_new_status: EnablementStatus,
    switch: &gtk::Switch,
    unit: &UnitInfo,
    is_dark: bool,
) {
    info!(
        "switch_ablement_state_set Unit \"{}\" enablement \"{}\" sw_active {} sw_state {} expected_new_status {expected_new_status}",
        unit.primary(),
        unit.enable_status_str(),
        switch.is_active(),
        switch.state()
    );

    let current_enabled_status = unit.enable_status_enum();

    if expected_new_status == current_enabled_status {
        set_switch_tooltip(current_enabled_status, switch, &unit.primary());
        return;
    }

    let switch = switch.clone();
    let control_panel = control_panel.clone();
    let unit = unit.clone();
    glib::spawn_future_local(async move {
        switch.set_sensitive(false);

        let unit_ = unit.clone();
        let enable_result =
            gio::spawn_blocking(move || systemd::disenable_unit_file(&unit_, expected_new_status))
                .await
                .expect("Task needs to finish successfully.");

        switch.set_sensitive(true);

        match enable_result {
            Ok(enablement_status_ret) => {
                let blue = blue(true).get_color();

                //
                let toast_info = format2!(
                    pgettext(
                        "toast",
                        "Unit <span fgcolor='{0}' font_family='monospace' size='larger'>{}</span> has been successfully <span fgcolor='{0}'>{}</span>"
                    ),
                    blue,
                    unit.primary(),
                    expected_new_status,
                );

                debug!("{toast_info} {:?}", enablement_status_ret);

                control_panel.add_toast_message(&toast_info, true);

                unit.set_enable_status(expected_new_status as u8);

                let enabled_new = expected_new_status == EnablementStatus::Enabled;
                switch.set_state(enabled_new);
            }

            Err(error) => {
                let error_message = match error {
                    SystemdErrors::SystemCtlError(s) => s,
                    _ => format!("{:?}", error),
                };
                let action_str = match expected_new_status {
                    EnablementStatus::Disabled => "Disabling",
                    EnablementStatus::Enabled => "Enabling",
                    _ => "???",
                };

                let blue = blue(is_dark).get_color();

                let toast_info = format!(
                    "{action_str} unit <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> has failed!",
                    unit.primary()
                );

                warn!("{toast_info} : {error_message}");

                control_panel.add_toast_message(&toast_info, true);
            }
        }

        handle_switch_sensivity(&switch, &unit, false);
    });
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
    let mut unit_file_state = unit.enable_status_enum();

    if check_current_state {
        let switch = switch.clone();
        let unit = unit.clone();
        glib::spawn_future_local(async move {
            let unit2 = unit.clone();
            let current_state =
                gio::spawn_blocking(move || match systemd::get_unit_file_state(&unit2) {
                    Ok(enblement_status) => enblement_status,
                    Err(err) => {
                        info!("Get unit state fail! For {:?} : {:?}", unit2.primary(), err);
                        unit_file_state
                    }
                })
                .await
                .expect("Task needs to finish successfully.");

            if current_state != unit_file_state {
                unit_file_state = current_state;
                unit.set_enable_status(unit_file_state as u8);
            }

            handle_switch_sensivity_part2(&switch, &unit, unit_file_state);
        });
    } else {
        handle_switch_sensivity_part2(switch, unit, unit_file_state);
    }
}

fn handle_switch_sensivity_part2(
    switch: &gtk::Switch,
    unit: &UnitInfo,
    unit_file_state: EnablementStatus,
) {
    if unit_file_state == EnablementStatus::Enabled
        || unit_file_state == EnablementStatus::EnabledRuntime
    {
        switch.set_state(true);
        switch.set_active(true);
    } else {
        switch.set_state(false);
        switch.set_active(false);
    }

    let sensitive = match unit_file_state {
        EnablementStatus::Enabled
        | EnablementStatus::EnabledRuntime
        | EnablementStatus::Disabled => {
            set_switch_tooltip(unit_file_state, switch, &unit.primary());
            true
        }
        _ => {
            switch.set_tooltip_text(None);
            false
        }
    };

    switch.set_sensitive(sensitive);
}
