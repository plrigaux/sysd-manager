use std::rc::Rc;

use gettextrs::pgettext;
use gtk::{gio, glib};
use tracing::{debug, info, warn};

use crate::format2;
use crate::systemd::{self, data::UnitInfo, enums::UnitFileStatus, errors::SystemdErrors};

use super::UnitControlPanel;
use crate::gtk::prelude::*;
use crate::utils::palette::{blue, dark_blue};

pub(super) fn switch_ablement_state_set(
    control_panel: &UnitControlPanel,
    expected_new_status: UnitFileStatus,
    switch: &gtk::Switch,
    unit: &UnitInfo,
    call_back: Rc<Box<dyn Fn()>>,
) {
    info!(
        "switch_ablement_state_set Unit \"{}\" enablement \"{}\" sw_active {} sw_state {} expected_new_status {expected_new_status}",
        unit.primary(),
        unit.enable_status().as_str(),
        switch.is_active(),
        switch.state()
    );

    let current_enabled_status = unit.enable_status();

    if expected_new_status == current_enabled_status {
        set_switch_tooltip(current_enabled_status, switch, &unit.primary());
        return;
    }

    let switch = switch.clone();
    let control_panel = control_panel.clone();
    let unit = unit.clone();

    //let call_back: Box<dyn Fn()> = Box::new(call_back.clone());
    glib::spawn_future_local(async move {
        switch.set_sensitive(false);

        let primary_name = unit.primary();
        let level = unit.dbus_level();
        let enable_status = unit.enable_status();
        let enable_result = gio::spawn_blocking(move || {
            systemd::disenable_unit_file(&primary_name, level, enable_status, expected_new_status)
        })
        .await
        .expect("Task needs to finish successfully.");

        switch.set_sensitive(true);

        match enable_result {
            Ok(enablement_status_ret) => {
                let blue = dark_blue().get_color();

                let toast_info = format2!(
                    //toast message on success
                    pgettext(
                        "toast",
                        "Unit <span fgcolor='{0}' font_family='monospace' size='larger'>{}</span> has been successfully <span fgcolor='{0}'>{}</span>"
                    ),
                    blue,
                    unit.primary(),
                    expected_new_status,
                );

                debug!("{toast_info} {enablement_status_ret:?}");

                control_panel.add_toast_message(&toast_info, true);

                unit.set_enable_status(expected_new_status);

                switch.set_state(expected_new_status == UnitFileStatus::Enabled);
            }

            Err(error) => {
                let error_message = match error {
                    SystemdErrors::SystemCtlError(s) => s,
                    _ => format!("{error:?}"),
                };

                let (action_str, action_log) = match expected_new_status {
                    UnitFileStatus::Disabled => {
                        //toast message action on fail
                        (pgettext("toast", "Disabling"), "Disabling")
                    }
                    UnitFileStatus::Enabled => {
                        //toast message action on fail
                        (pgettext("toast", "Enabling"), "Enabling")
                    }
                    _ => ("???".to_owned(), "???"),
                };

                let blue = blue().get_color();

                let toast_info = format2!(
                    //toast message on fail, arg0 : Enabling/Disabling, arg1 : unit name
                    pgettext("toast", "{} unit {} has failed!"),
                    action_str,
                    format!(
                        "<span fgcolor='{}' font_family='monospace' size='larger'>{}</span> ",
                        blue,
                        unit.primary()
                    )
                );

                warn!(
                    "{action_log} unit {} has Failed! : {error_message}",
                    unit.primary()
                );

                control_panel.add_toast_message(&toast_info, true);
            }
        }

        handle_switch_sensivity(&switch, &unit, false);

        call_back()
    });
}

pub(super) fn reeenable_unit(
    control_panel: &UnitControlPanel,
    switch: &gtk::Switch,
    unit: &UnitInfo,
    call_back: Rc<Box<dyn Fn()>>,
) {
    let expected_new_status = unit.enable_status(); //Expect new status
    info!(
        "Reeenable unit Unit \"{}\" enablement \"{}\" sw_active {} sw_state {} expected_new_status {expected_new_status}",
        unit.primary(),
        unit.enable_status().as_str(),
        switch.is_active(),
        switch.state()
    );

    let switch = switch.clone();
    let control_panel = control_panel.clone();
    let unit = unit.clone();

    //let call_back: Box<dyn Fn()> = Box::new(call_back.clone());
    glib::spawn_future_local(async move {
        switch.set_sensitive(false);

        let primary_name = unit.primary();
        let level = unit.dbus_level();

        let enable_result = gio::spawn_blocking(move || {
            systemd::disenable_unit_file(
                &primary_name,
                level,
                UnitFileStatus::Enabled,
                UnitFileStatus::Disabled,
            )
            .map(|_ret| {
                systemd::disenable_unit_file(
                    &primary_name,
                    level,
                    UnitFileStatus::Disabled,
                    expected_new_status,
                )
            })
        })
        .await
        .expect("Task needs to finish successfully.");

        switch.set_sensitive(true);

        match enable_result {
            Ok(enablement_status_ret) => {
                let blue = dark_blue().get_color();

                //Toast message action on Reenable, in the rentance: ... has been successfully "Reenable"
                let action_str = pgettext("toast", "Reenable");

                let toast_info = format2!(
                    //toast message on success
                    pgettext(
                        "toast",
                        "Unit <span fgcolor='{0}' font_family='monospace' size='larger'>{}</span> has been successfully <span fgcolor='{0}'>{}</span>"
                    ),
                    blue,
                    unit.primary(),
                    action_str,
                );

                debug!("{toast_info} {enablement_status_ret:?}");

                control_panel.add_toast_message(&toast_info, true);

                unit.set_enable_status(expected_new_status);

                switch.set_state(expected_new_status == UnitFileStatus::Enabled);
            }

            Err(error) => {
                let error_message = match error {
                    SystemdErrors::SystemCtlError(s) => s,
                    _ => format!("{error:?}"),
                };

                //toast message action on fail
                let action_str = pgettext("toast", "Reenabling");

                let blue = blue().get_color();

                let toast_info = format2!(
                    //toast message on fail
                    pgettext(
                        "toast",
                        "{} unit <span fgcolor='{0}' font_family='monospace' size='larger'>{}</span> has failed!"
                    ),
                    blue,
                    action_str,
                    unit.primary()
                );

                warn!("{toast_info} : {error_message}");

                control_panel.add_toast_message(&toast_info, true);
            }
        }

        handle_switch_sensivity(&switch, &unit, false);

        call_back()
    });
}

//TODO do function to more constitency
fn set_switch_tooltip(enabled: UnitFileStatus, switch: &gtk::Switch, unit_name: &str) {
    let enabled = enabled == UnitFileStatus::Enabled;

    let action_text = if enabled {
        pgettext("controls", "Disable unit {}")
    } else {
        pgettext("controls", "Enable unit {}")
    };

    let blue = blue().get_color();
    let unit_str = format!(
        "<span fgcolor='{}' font_family='monospace' size='larger' weight='bold'>{}</span>",
        blue, unit_name
    );

    let tooltip = format2!(action_text, unit_str);

    switch.set_tooltip_markup(Some(&tooltip));
}

pub(super) fn handle_switch_sensivity(
    switch: &gtk::Switch,
    unit: &UnitInfo,
    check_current_state: bool,
) {
    let mut unit_file_state = unit.enable_status();

    if check_current_state {
        let switch = switch.clone();
        let unit = unit.clone();

        let primary_name = unit.primary();
        let level = unit.dbus_level();
        glib::spawn_future_local(async move {
            let current_state = gio::spawn_blocking(move || {
                systemd::get_unit_file_state(level, &primary_name)
                    .inspect_err(|err| {
                        info!(
                            "Get unit file state fail! For {:?} : {:?}",
                            primary_name, err
                        )
                    })
                    .unwrap_or(unit_file_state)
            })
            .await
            .expect("Task needs to finish successfully.");

            if current_state != unit_file_state {
                unit_file_state = current_state;
                unit.set_enable_status(unit_file_state);
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
    unit_file_state: UnitFileStatus,
) {
    if unit_file_state == UnitFileStatus::Enabled
        || unit_file_state == UnitFileStatus::EnabledRuntime
    {
        switch.set_state(true);
        switch.set_active(true);
    } else {
        switch.set_state(false);
        switch.set_active(false);
    }

    let sensitive = match unit_file_state {
        UnitFileStatus::Enabled | UnitFileStatus::EnabledRuntime | UnitFileStatus::Disabled => {
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
