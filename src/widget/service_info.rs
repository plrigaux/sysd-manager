use crate::{errors::SysDManagerErrors, systemd::data::UnitInfo};

use super::info_window::InfoWindow;
use crate::gtk::prelude::*;

pub fn build_service_status(unit : &UnitInfo) -> Result<gtk::Widget, SysDManagerErrors> {

    let builder = gtk::Builder::from_resource("/io/github/plrigaux/sysd-manager/service_status.ui");

    let id_name = "service_layout";
    let Some(grid) = builder.object::<gtk::Grid>(id_name) else {
        return Err(SysDManagerErrors::GTKBuilderObjectNotfound(
            id_name.to_owned(),
        ));
    };

    let id_name ="name_description";
    let Some(name_description) = builder.object::<gtk::Label>(id_name) else {
        return Err(SysDManagerErrors::GTKBuilderObjectNotfound(
            id_name.to_owned(),
        ));
    };


    let nd = format!("{} - {}", unit.primary(), unit.description());
    name_description.set_label(&nd);

    let info_window = InfoWindow::new();

    info_window.fill_data(&unit);

    info_window.present();

    Ok(grid.into())
}