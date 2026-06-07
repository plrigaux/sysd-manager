use adw::prelude::NavigationPageExt;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

use crate::widget::creator::{
    PageType, UnitCreatorWindow, unit_file_creator_page::UnitFileCreatorPage,
};

glib::wrapper! {

    pub struct ServiceCreatorPage(ObjectSubclass<imp::ServiceCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl ServiceCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>, page: PageType) -> Self {
        let obj: ServiceCreatorPage = glib::Object::new();
        obj.set_tag(Some(page.id()));
        let _ = obj.imp().window.set(window);
        // obj.imp().update_from_unit_info();
        obj
    }

    pub fn update_view(&self, page: &UnitFileCreatorPage) {
        self.imp().update_view(page);
    }

    pub fn update_from_file_content(&self, content: &str) {
        self.imp().update_from_file_content(content);
    }

    pub fn file_content(&self) -> String {
        self.imp().file_content()
    }
}

mod imp {

    use super::*;
    use crate::widget::creator::{CreateUnitErr, UnitCreateType, unit_file::UnitFileData};
    use adw::{
        prelude::{ComboRowExt, PreferencesRowExt},
        subclass::prelude::*,
    };
    use gtk::{StringObject, glib, prelude::*};
    use std::{
        cell::{Cell, OnceCell, RefCell},
        fs,
        os::unix::fs::PermissionsExt,
        path::Path,
    };
    use tracing::warn;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/service_creator_page.ui")]
    #[properties(wrapper_type = super::ServiceCreatorPage)]
    pub struct ServiceCreatorPageImp {
        #[property(get, set, default)]
        creation_type: Cell<UnitCreateType>,

        #[template_child]
        description_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        exec_start_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        working_directory_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        restart_policy_combo: TemplateChild<adw::ComboRow>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

        pub(super) file_data: RefCell<UnitFileData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceCreatorPageImp {
        const NAME: &'static str = "ServiceCreatorPage";
        type Type = ServiceCreatorPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServiceCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();

            let event_foc = gtk::EventControllerFocus::new();
            event_foc.connect_leave(|event| {
                if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                    // let text = entry.text();
                    ServiceCreatorPageImp::validate_entry_strat(entry);
                }
            });
            self.exec_start_entry.add_controller(event_foc);

            let vec = vec![
                "",
                "always",
                "on-success",
                "on-failure",
                "on-abnormal",
                "on-abort",
                "on-watchdog",
            ];

            let model = gtk::StringList::new(&vec);

            self.restart_policy_combo.set_model(Some(&model));

            self.description_entry
                .connect_has_focus_notify(|entry| entry.select_region(0, -1));
            self.description_entry
                .connect_focus_on_click_notify(|entry| entry.select_region(0, -1));
            self.exec_start_entry
                .connect_has_focus_notify(|entry| entry.select_region(0, -1));
            self.exec_start_entry
                .connect_focus_on_click_notify(|entry| entry.select_region(0, -1));
            self.working_directory_entry
                .connect_has_focus_notify(|entry| entry.select_region(0, -1));
            self.working_directory_entry
                .connect_focus_on_click_notify(|entry| entry.select_region(0, -1));
        }
    }

    impl ServiceCreatorPageImp {
        fn validate_entry_strat(entry: &adw::EntryRow) {
            let text = entry.text();

            let name_err = match get_file_path(text.as_str()) {
                Ok(text) => {
                    if text.is_empty() {
                        CreateUnitErr::NoErr
                    } else {
                        let path = Path::new(text);

                        if !path.exists() {
                            CreateUnitErr::FileNotExits
                        } else if !path.is_file() {
                            CreateUnitErr::NotFile
                        } else if !is_executable(path) {
                            CreateUnitErr::NotExecutable
                        } else {
                            CreateUnitErr::NoErr
                        }
                    }
                }

                Err(err) => err,
            };

            match name_err {
                CreateUnitErr::NoErr => {
                    entry.remove_css_class("warning");
                }
                _ => {
                    entry.add_css_class("warning");
                }
            }
            let prefix = "ExecStart";
            entry.set_title(&name_err.title_err(prefix));
        }
    }

    fn is_executable(path: &Path) -> bool {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };

        metadata.permissions().mode() & 0o111 != 0
    }

    #[gtk::template_callbacks]
    impl ServiceCreatorPageImp {
        #[template_callback]
        fn working_directory_search_dialog_clicked(&self, _button: gtk::Button) {
            let file_dialog = gtk::FileDialog::builder()
                .title("Select a working directory")
                .accept_label("Select")
                .build();

            let create_service_page = self.obj().clone();

            let text = self.working_directory_entry.text();
            let text = get_file_path(&text).unwrap_or_default();
            if text.is_empty() {
                set_initial_folder(&file_dialog);
            } else {
                let path = Path::new(text);
                if path.exists() {
                    let file = gio::File::for_path(path);
                    file_dialog.set_initial_file(Some(&file));
                } else {
                    println!("not ex {text}");
                    set_initial_folder(&file_dialog);
                }
            }

            let win = self.window.get().and_then(|w| w.upgrade());
            let win = win.and_upcast_ref::<gtk::Window>();

            file_dialog.select_folder(win, None::<&gio::Cancellable>, move |result| match result {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        let file_path_str = path.display().to_string();
                        create_service_page
                            .imp()
                            .working_directory_entry
                            .set_text(&file_path_str);
                    }
                }
                Err(e) => warn!("Unit File Selection Error {e:?}"),
            });
        }

        #[template_callback]
        fn exec_start_dialog_clicked(&self, _button: gtk::Button) {
            let file_dialog = gtk::FileDialog::builder()
                .title("Select executable")
                .accept_label("Select")
                .build();

            let create_service_page = self.obj().clone();

            let text = self.exec_start_entry.text();
            let text = get_file_path(&text).unwrap_or_default();
            if text.is_empty() {
                set_initial_folder(&file_dialog);
            } else {
                let path = Path::new(text);
                if path.exists() {
                    let file = gio::File::for_path(path);
                    if path.is_dir() {
                        // println!("dir {:?} ", path);
                        file_dialog.set_initial_folder(Some(&file));
                    } else if path.is_file() {
                        // println!("file {:?} ", path);
                        file_dialog.set_initial_file(Some(&file));
                    }
                } else {
                    // println!("not ex");
                    set_initial_folder(&file_dialog);
                }
            }

            let win = self.window.get().and_then(|w| w.upgrade());
            let win = win.and_upcast_ref::<gtk::Window>();

            file_dialog.open(win, None::<&gio::Cancellable>, move |result| match result {
                Ok(file) => {
                    if let Some(path) = file.path() {
                        let mut file_path_str = path.display().to_string();
                        escape(&mut file_path_str);
                        create_service_page
                            .imp()
                            .exec_start_entry
                            .set_text(&file_path_str);
                    }
                }
                Err(e) => warn!("Unit File Selection Error {e:?}"),
            });
        }

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

            file_data.set_description(self.description_entry.text());
            file_data.set_working_directory(self.working_directory_entry.text());
            file_data.set_exec_start(self.exec_start_entry.text());

            let restart = self
                .restart_policy_combo
                .selected_item()
                .and_downcast_ref::<gtk::StringObject>()
                .map(|s| s.string());
            file_data.set_restart(restart.unwrap_or_default());
            file_data.sort();
        }

        pub fn update_from_file_content(&self, content: &str) {
            let Some(data) = UnitFileData::from_content(content) else {
                return;
            };

            self.description_entry.set_text(data.description());
            self.working_directory_entry
                .set_text(data.working_directory());
            self.exec_start_entry.set_text(data.exec_start());

            let restart = data.restart();
            let mut position_sel = 0;
            if !restart.is_empty()
                && let Some(list_model) = self.restart_policy_combo.model()
            {
                //TODO make a map if too slow
                for position in 0..list_model.n_items() {
                    if let Some(string_item) = list_model
                        .item(position)
                        .and_downcast_ref::<StringObject>()
                        .map(|s| s.string())
                        && string_item.as_str() == restart
                    {
                        position_sel = position;
                        break;
                    }
                }
            }
            self.restart_policy_combo.set_selected(position_sel);

            self.file_data.replace(data);
        }
    }

    impl WidgetImpl for ServiceCreatorPageImp {}

    impl NavigationPageImpl for ServiceCreatorPageImp {}

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
        use super::*;

        #[test]
        fn test_get_file() {
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
}
