use super::*;
use crate::{
    upgrade, upgrade_opt,
    widget::creator::{
        CreateUnitErr, creator_page_mount::standard_output::output_file_descriptor,
        suggestion::SuggestionRow, unit_file::UnitFileData,
    },
};
use adw::{
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt},
    subclass::prelude::*,
};
use glib::object::Cast;
use indexmap::{IndexMap, map::Entry};
use regex::Regex;
use std::{
    cell::{OnceCell, RefCell},
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
};
use tracing::info;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator_page_mount.ui")]
#[properties(wrapper_type = super::CreatorPageMount)]
pub struct CreatorPageMountImp {
    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    pub(super) file_data: RefCell<UnitFileData>,

    pub(super) widget_track: RefCell<IndexMap<String, Vec<gtk::Widget>>>,

    validate_cpu_quota_regex: OnceCell<Regex>,
    validate_memory_high_regex: OnceCell<Regex>,
}

#[glib::object_subclass]
impl ObjectSubclass for CreatorPageMountImp {
    const NAME: &'static str = "CreatorPageMount";
    type Type = CreatorPageMount;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        SuggestionRow::default();
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for CreatorPageMountImp {
    fn constructed(&self) {
        self.parent_constructed();

        let event_focus = gtk::EventControllerFocus::new();
    }
}

impl CreatorPageMountImp {}

#[gtk::template_callbacks]
impl CreatorPageMountImp {
    #[template_callback]
    fn environment_add_clicked(&self, _button: gtk::Button) {}
    #[template_callback]
    fn exec_start_dialog_clicked(&self, _button: gtk::Button) {}
}

impl CreatorPageMountImp {
    pub(super) fn update_view(&self, page: &UnitFileCreatorPage) {
        self.fill_data();
        let data = self.file_data.borrow();
        page.update_view(&data);
    }

    pub(super) fn file_content(&self) -> String {
        self.fill_data();
        self.file_data.borrow().to_file()
    }

    fn fill_data(&self) {
        let mut file_data = self.file_data.borrow_mut();

        //        file_data.set_description(self.description_entry.text());

        file_data.sort();
    }

    pub fn update_from_file_content(&self, content: &str) {
        let Some(data) = UnitFileData::from_content(content) else {
            return;
        };

        self.file_data.replace(data);
    }
}

impl WidgetImpl for CreatorPageMountImp {}

impl NavigationPageImpl for CreatorPageMountImp {}

fn escape(file_path: &mut String) {
    if file_path.contains(char::is_whitespace) {
        file_path.insert(0, '"');
        file_path.push('"');
    }
}

fn set_initial_folder(file_dialog: &gtk::FileDialog) {
    if let Ok(home) = std::env::var("HOME") {
        let path = Path::new(&home);
        let dir = gio::File::for_path(path);
        file_dialog.set_initial_folder(Some(&dir));
    }
}

fn get_file_path(text: &str) -> Result<&str, CreateUnitErr> {
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
}
