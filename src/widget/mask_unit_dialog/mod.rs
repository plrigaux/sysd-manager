mod imp;

use gtk::{
    glib::{self},
    subclass::prelude::ObjectSubclassIsExt,
};
use log::warn;

use crate::systemd::{self, data::UnitInfo, errors::SystemdErrors};

use super::{InterPanelMessage, app_window::AppWindow, unit_control_panel::UnitControlPanel};

// ANCHOR: mod
glib::wrapper! {
    pub struct MaskUnitDialog(ObjectSubclass<imp::MaskUnitDialogImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MaskUnitDialog {
    pub fn new(
        unit: Option<&UnitInfo>,
        is_dark: bool,
        app_window: Option<&AppWindow>,
        unit_control: &UnitControlPanel,
    ) -> Self {
        let obj: MaskUnitDialog = glib::Object::new();
        let imp = obj.imp();
        imp.set_app_window(app_window, unit_control);
        imp.set_unit(unit);
        imp.set_inter_message(&InterPanelMessage::IsDark(is_dark));

        obj
    }
}

pub fn after_mask(
    _method_name: &str,
    unit: Option<&UnitInfo>,
    result: Result<(), SystemdErrors>,
    control: &UnitControlPanel,
) {
    if result.is_err() {
        return;
    }

    let Some(unit) = unit else {
        warn!("Unit None");
        return;
    };

    let unit = unit.clone();
    let control = control.clone();
    glib::spawn_future_local(async move {
        let unit2 = unit.clone();

        let (sender, receiver) = tokio::sync::oneshot::channel();

        crate::systemd::runtime().spawn(async move {
            let response = systemd::complete_single_unit_information(&unit2).await;

            sender
                .send(response)
                .expect("The channel needs to be open.");
        });

        let vec_unit_info = match receiver.await.expect("Tokio receiver works") {
            Ok(unit_files) => unit_files,
            Err(err) => {
                warn!("Fail to update Unit info {:?}", err);
                return Err(err);
            }
        };

        if let Some(update) = vec_unit_info.into_iter().next() {
            unit.update_from_unit_info(update);
        }

        control.selection_change(Some(&unit));
        Ok::<(), SystemdErrors>(())
    });
}
