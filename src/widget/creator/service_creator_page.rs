use crate::widget::creator::{
    PageType, UnitCreatorWindow, unit_file_creator_page::UnitFileCreatorPage,
};
use adw::prelude::NavigationPageExt;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

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

    pub fn update_from_unit_info(&self) {
        self.imp().update_from_unit_info();
    }
}

pub const ENVIRONMENT: &str = "Environment";
mod imp {

    use super::*;
    use crate::{
        upgrade, upgrade_opt,
        widget::creator::{CreateUnitErr, UnitCreateType, unit_file::UnitFileData},
    };
    use adw::{
        prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt},
        subclass::prelude::*,
    };
    use gettextrs::{gettext, pgettext};
    use gtk::{StringObject, glib, prelude::*};
    use indexmap::{IndexMap, map::Entry};
    use itertools::{EitherOrBoth, Itertools};
    use regex::Regex;
    use std::{
        cell::{Cell, OnceCell, RefCell},
        fs,
        os::unix::fs::PermissionsExt,
        path::Path,
    };
    use tracing::{info, warn};

    const VALIDATE_CPU_QUOTA_REGEX: &str = r"\d+%";
    const VALIDATE_MEMORY_HIGH_REGEX: &str = r"^[1-9][0-9]*[%KMGT]?$";

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
        environment_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        working_directory_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        restart_policy_combo: TemplateChild<adw::ComboRow>,

        #[template_child]
        unit_wants: TemplateChild<adw::ComboRow>,

        #[template_child]
        unit_after: TemplateChild<adw::ComboRow>,

        #[template_child]
        service_group: TemplateChild<adw::PreferencesGroup>,

        #[template_child]
        memory_high_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        cpu_quota_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        user_combo: TemplateChild<adw::ComboRow>,

        #[template_child]
        group_combo: TemplateChild<adw::ComboRow>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

        pub(super) file_data: RefCell<UnitFileData>,

        pub(super) widget_track: RefCell<IndexMap<String, Vec<gtk::Widget>>>,

        validate_cpu_quota_regex: OnceCell<Regex>,
        validate_memory_high_regex: OnceCell<Regex>,
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

            let event_focus = gtk::EventControllerFocus::new();
            event_focus.connect_leave(|event| {
                if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                    ServiceCreatorPageImp::validate_entry_strat(entry);
                }
            });
            self.exec_start_entry.add_controller(event_focus);

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

            self.add_track(ENVIRONMENT, &self.environment_entry.get());

            let users = unsafe { uzers::all_users() };

            let mut users: Vec<_> = users
                .map(|u| u.name().to_string_lossy().into_owned())
                .collect();

            users.push("".into());
            users.sort();

            let model = gtk::StringList::new(&[]);
            for user in users {
                model.append(&user);
            }
            self.user_combo.set_model(Some(&model));

            let groups = unsafe { uzers::all_groups() };

            let mut groups: Vec<_> = groups
                .map(|u| u.name().to_string_lossy().into_owned())
                .collect();

            groups.push("".into());
            groups.sort();

            let model = gtk::StringList::new(&[]);
            for group in groups {
                model.append(&group);
            }
            self.group_combo.set_model(Some(&model));

            let this = self.obj().clone();
            let event_focus = gtk::EventControllerFocus::new();
            event_focus.connect_leave(move |event| {
                if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                    ServiceCreatorPageImp::validate_cpu_quota(this.imp(), entry);
                }
            });

            self.cpu_quota_entry.add_controller(event_focus);

            let this = self.obj().clone();
            let event_focus = gtk::EventControllerFocus::new();
            event_focus.connect_leave(move |event| {
                if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                    ServiceCreatorPageImp::validate_memory_high(this.imp(), entry);
                }
            });

            self.memory_high_entry.add_controller(event_focus);

            let this = self.obj().clone();
            let event_focus = gtk::EventControllerFocus::new();
            event_focus.connect_leave(move |event| {
                if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                    this.imp().validate_working_directory(entry);
                }
            });

            self.working_directory_entry.add_controller(event_focus);
        }
    }

    impl ServiceCreatorPageImp {
        fn validate_entry_strat(entry: &adw::EntryRow) {
            let text = entry.text();

            let name_err = if text.is_empty() {
                CreateUnitErr::NoErr
            } else {
                let path = Path::new(&text);

                if !path.exists() {
                    CreateUnitErr::FileNotExits
                } else if !path.is_file() {
                    CreateUnitErr::NotFile
                } else if !is_executable(path) {
                    CreateUnitErr::NotExecutable
                } else if !path.is_absolute() {
                    CreateUnitErr::NotAbsolute
                } else {
                    CreateUnitErr::NoErr
                }
            };

            Self::apply_validation_result(entry, name_err, "WorkingDirectory");
        }

        fn validate_working_directory(&self, entry: &adw::EntryRow) {
            let text = entry.text();

            let name_err = match get_file_path(text.as_str()) {
                Ok(text) => {
                    if text.is_empty() {
                        CreateUnitErr::NoErr
                    } else {
                        let path = Path::new(text);

                        if !path.exists() {
                            CreateUnitErr::FileNotExits
                        } else if !path.is_dir() {
                            CreateUnitErr::NotDir
                        } else if !is_executable(path) {
                            CreateUnitErr::NotExecutable
                        } else if !path.is_absolute() {
                            CreateUnitErr::NotAbsolute
                        } else {
                            CreateUnitErr::NoErr
                        }
                    }
                }

                Err(err) => err,
            };

            Self::apply_validation_result(entry, name_err, "ExecStart");
        }

        fn cpu_quota_regex(&self) -> &Regex {
            self.validate_cpu_quota_regex
                .get_or_init(|| Regex::new(VALIDATE_CPU_QUOTA_REGEX).unwrap())
        }

        fn validate_cpu_quota(&self, entry: &adw::EntryRow) {
            let text = entry.text();
            let cpuquota = "CPUQuota";
            info!("Validating {:?} {:?}", cpuquota, text);

            let name_err = if text.is_empty() {
                CreateUnitErr::NoErr
            } else if !self.cpu_quota_regex().is_match(text.as_str()) {
                CreateUnitErr::Malformed
            } else {
                CreateUnitErr::NoErr
            };

            Self::apply_validation_result(entry, name_err, cpuquota);
        }

        fn get_memory_high_regex(&self) -> &Regex {
            self.validate_memory_high_regex
                .get_or_init(|| Regex::new(VALIDATE_MEMORY_HIGH_REGEX).unwrap())
        }

        fn validate_memory_high(&self, entry: &adw::EntryRow) {
            let text = entry.text();
            let cpuquota = "MemoryHigh";
            info!("Validating {:?} {:?}", cpuquota, text);

            let name_err = if text.is_empty() {
                CreateUnitErr::NoErr
            } else if !self.get_memory_high_regex().is_match(text.as_str()) {
                CreateUnitErr::Malformed
            } else {
                CreateUnitErr::NoErr
            };

            Self::apply_validation_result(entry, name_err, cpuquota);
        }

        fn apply_validation_result(entry: &adw::EntryRow, name_err: CreateUnitErr, prefix: &str) {
            match name_err {
                CreateUnitErr::NoErr => {
                    entry.remove_css_class("warning");
                }
                _ => {
                    entry.add_css_class("warning");
                }
            }
            entry.set_title(&name_err.title_err(prefix));
        }

        fn add_track(&self, param: &str, widget: &impl IsA<gtk::Widget>) {
            match self.widget_track.borrow_mut().entry(param.to_owned()) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().push(widget.as_ref().clone())
                }
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert_entry(vec![widget.as_ref().clone()]);
                }
            };
        }

        fn remove_track(&self, param: &str, widget: &impl IsA<gtk::Widget>) {
            if let Some(v) = self.widget_track.borrow_mut().get_mut(param) {
                for (i, w) in v.iter().enumerate() {
                    if w == widget {
                        v.remove(i);
                        break;
                    }
                }
            };
        }

        pub(crate) fn update_from_unit_info(&self) {
            let window = upgrade_opt!(self.window.get());

            let model = window.imp().get_trigger_units_model();

            self.unit_wants.set_model(Some(&model));
            self.unit_after.set_model(Some(&model));
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
                .title(pgettext("unit creation", "Select executable"))
                .accept_label(gettext("Select"))
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

        #[template_callback]
        fn environment_add_clicked(&self, _button: gtk::Button) {
            self.add_entry(ENVIRONMENT);
        }

        fn last_of(&self, id: &str) -> Option<gtk::Widget> {
            if let Some(w_list) = self.widget_track.borrow_mut().get(id)
                && let Some(w) = w_list.last()
            {
                Some(w.clone())
            } else {
                match id {
                    ENVIRONMENT => Some(self.environment_entry.get().into()),
                    _ => None,
                }
            }
        }

        fn add_entry(&self, id: &str) -> adw::EntryRow {
            let entry = adw::EntryRow::builder().title(id).build();

            let mut vec = Vec::new();
            let mut env_idx = 0;

            let last_of = self.last_of(id);
            for i in 0..10_000 {
                let Some(widget) = self.service_group.row(i) else {
                    break;
                };

                if Some(&widget) == last_of.as_ref() {
                    env_idx = i;
                }

                vec.push(widget);
            }

            //Remove next widget for inserting the new one
            for (i, w) in vec.iter().enumerate().rev() {
                if env_idx == i as u32 {
                    break;
                } else {
                    self.service_group.remove(w);
                }
            }

            //Insert new
            self.service_group.add(&entry);
            self.add_track(id, &entry);

            //Add other widget
            for w in vec.iter().skip(env_idx as usize + 1) {
                self.service_group.add(w);
            }

            entry
        }

        fn remove_entry(&self, id: &str, w: &gtk::Widget) {
            self.service_group.remove(w);
            self.remove_track(id, w);
        }
    }

    impl ServiceCreatorPageImp {
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

            let environments = self.widget_track.borrow().get(ENVIRONMENT).map(|v| {
                v.iter()
                    .filter_map(|widget| {
                        widget
                            .downcast_ref::<adw::EntryRow>()
                            .map(|entry| entry.text())
                    })
                    .collect::<Vec<glib::GString>>()
            });
            file_data.set_environment(environments.as_deref());
            file_data.set_exec_start(self.exec_start_entry.text());
            file_data.set_user(self.user_combo.subtitle());
            file_data.set_group(self.group_combo.subtitle());
            file_data.set_working_directory(self.working_directory_entry.text());
            file_data.set_after(self.unit_after.subtitle());
            file_data.set_wants(self.unit_wants.subtitle());
            file_data.set_cpu_quota(self.cpu_quota_entry.text());
            file_data.set_memory_high(self.memory_high_entry.text());

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
            self.unit_after.set_subtitle(data.after());
            self.unit_wants.set_subtitle(data.wants());

            if let Some(env) = data.environment() {
                let mut to_remove = Vec::new();
                let mut to_add = Vec::new();
                {
                    let w_track = self.widget_track.borrow();
                    let w_it = w_track
                        .get(ENVIRONMENT)
                        .map(|v| v.iter())
                        .unwrap_or_default();
                    for (i, v) in env.iter().zip_longest(w_it).enumerate() {
                        match v {
                            EitherOrBoth::Both(s, w) => {
                                let Some(entry) = w.downcast_ref::<adw::EntryRow>() else {
                                    warn!("bad downcast");
                                    continue;
                                };
                                entry.set_text(s.as_str());
                            }
                            EitherOrBoth::Left(s) => {
                                to_add.push(s);
                            }
                            EitherOrBoth::Right(w) => {
                                if i != 0 {
                                    to_remove.push(w.clone())
                                } else {
                                    self.environment_entry.set_text("");
                                }
                            }
                        };
                    }
                }
                for s in to_add {
                    self.add_entry(ENVIRONMENT).set_text(s.as_str());
                }
                for w in to_remove {
                    self.remove_entry(ENVIRONMENT, &w);
                }
            } else {
                self.environment_entry.set_text("");
            }

            self.exec_start_entry.set_text(data.exec_start());
            self.working_directory_entry
                .set_text(data.working_directory());

            self.cpu_quota_entry.set_text(data.cpu_quota());
            self.memory_high_entry.set_text(data.memory_high());

            self.user_combo.set_subtitle(data.user());
            self.group_combo.set_subtitle(data.group());

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

        #[test]
        fn test_cpu_quota() {
            init_logs();
            let re = Regex::new(VALIDATE_CPU_QUOTA_REGEX).unwrap();

            assert!(re.is_match("50%"));
            assert!(!re.is_match("50"));
            assert!(re.is_match("33.33%"));
        }

        #[test]
        fn test_memory_hight() {
            init_logs();
            let re = Regex::new(VALIDATE_MEMORY_HIGH_REGEX).unwrap();

            assert!(re.is_match("50K"));
            assert!(!re.is_match("50Kb"));
            assert!(re.is_match("50%"));
            assert!(re.is_match("50"));
            assert!(!re.is_match("33.22"));
        }
    }
}
