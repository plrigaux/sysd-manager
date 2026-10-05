use std::path::Path;

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

pub fn set_initial_folder(file_dialog: &gtk::FileDialog) {
    if let Ok(home) = std::env::var("HOME") {
        let path = Path::new(&home);
        let dir = gio::File::for_path(path);
        file_dialog.set_initial_folder(Some(&dir));
    }
}

pub fn get_file_path(text: &str) -> Result<&str, CreateUnitErr> {
    let text = text.trim_start();
    let mut begin = 0;
    let mut end = text.len();
    let mut in_quotes = false;

    for (idx, char) in text.char_indices() {
        if char.is_whitespace() && !in_quotes {
            end = idx;
            break;
        } else if char == '"' {
            if idx == 0 {
                in_quotes = true;
                begin = 1;
            } else {
                end = idx;
                in_quotes = false;
                break;
            }
        }
    }
    if in_quotes {
        return Err(CreateUnitErr::Malformed);
    }
    Ok(&text[begin..end])
}

#[cfg(test)]
mod tests {
    use test_base::init_logs;

    use super::*;

    #[test]
    fn test_get_file() {
        init_logs();
        assert_eq!(get_file_path("text"), Ok("text"));
        assert_eq!(get_file_path("  text"), Ok("text"));
        assert_eq!(get_file_path("  text   "), Ok("text"));
        assert_eq!(get_file_path("  text -f  "), Ok("text"));
        assert_eq!(get_file_path(r#""text asdf" xxx"#), Ok("text asdf"));
        assert_eq!(get_file_path("\"\"text"), Ok(""));

        assert_eq!(
            get_file_path("/home/plr/bin/AppDir/etc"),
            Ok("/home/plr/bin/AppDir/etc")
        );
    }
}
