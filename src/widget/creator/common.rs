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

pub fn get_file_path_in_quotes(text: &str) -> &str {
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
                break;
            }
        }
    }

    &text[begin..end]
}

#[cfg(test)]
mod tests {
    use test_base::init_logs;

    use super::*;

    #[test]
    fn test_get_file_path() {
        init_logs();
        assert_eq!(get_file_path_in_quotes("text"), "text");
        assert_eq!(get_file_path_in_quotes("  text"), "text");
        assert_eq!(get_file_path_in_quotes("  text   "), "text");
        assert_eq!(get_file_path_in_quotes("  text -f  "), "text");
        assert_eq!(get_file_path_in_quotes(r#""text asdf" xxx"#), "text asdf");
        assert_eq!(get_file_path_in_quotes("\"\"text"), "");
        assert_eq!(get_file_path_in_quotes(" \"file path\" -1"), "file path");
        //Linux can have tailing spaces
        assert_eq!(
            get_file_path_in_quotes("       \"   file path \" -1"),
            "   file path "
        );

        assert_eq!(
            get_file_path_in_quotes("/home/plr/bin/AppDir/etc"),
            "/home/plr/bin/AppDir/etc"
        );
    }
}
