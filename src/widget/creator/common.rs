use glib::object::IsA;
use tracing::error;

use crate::widget::creator::{CreateUnitErr, UnitCreatorWindow};

pub fn validate_unit_common(
    preference: &(impl IsA<adw::PreferencesRow> + gtk::prelude::WidgetExt),
    prefix: &str,
    unit_name: Option<&str>,
    window: &glib::WeakRef<UnitCreatorWindow>,
) {
    let unit_name = unit_name.map(|s| s.trim()).unwrap_or_default();
    let name_err = if unit_name.is_empty() {
        CreateUnitErr::NoErr
    } else {
        match is_fill_exist(window, unit_name) {
            Ok(true) => CreateUnitErr::NoErr,
            Ok(false) => CreateUnitErr::NotUnit,
            Err(e) => e,
        }
    };

    CreateUnitErr::apply_validation_result(preference, &name_err, prefix);
}

pub(crate) fn is_fill_exist(
    window: &glib::WeakRef<UnitCreatorWindow>,
    unit_name: &str,
) -> Result<bool, CreateUnitErr> {
    let Some(window) = window.upgrade() else {
        error!("No parent window");
        return Err(CreateUnitErr::Unknown);
    };

    Ok(window.unit_file_list()?.contains(unit_name))
}
