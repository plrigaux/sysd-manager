use crate::{
    upgrade, upgrade_opt,
    widget::{
        self,
        creator::{
            self, CreateUnitErr, UnitCreatorWindow, common,
            creator_page_mount::{CreatorPageMount, mount_tools, validator},
            suggestion::SuggestionRow,
            unit_file::UnitFileData,
            unit_file_creator_page::UnitFileCreatorPage,
        },
    },
};
use adw::{prelude::*, subclass::prelude::*};
use gettextrs::pgettext;
use glib::WeakRef;
use regex::Regex;
use std::{
    cell::{OnceCell, RefCell},
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use systemd::runtime;
use tracing::{debug, info, warn};

const DIRECTORYMODE: &str = "DirectoryMode";
const DMODE_MAX: usize = 4;
#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/creator_page_mount.ui")]
#[properties(wrapper_type = super::CreatorPageMount)]
pub struct CreatorPageMountImp {
    #[template_child]
    mount_type_suggestion: TemplateChild<SuggestionRow>,

    #[template_child]
    description_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    what_entry: TemplateChild<SuggestionRow>,

    #[template_child]
    where_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    mount_options_entry: TemplateChild<adw::EntryRow>,

    #[template_child]
    mount_avanced_group: TemplateChild<adw::PreferencesGroup>,

    #[template_child]
    directory_mode_entry: TemplateChild<adw::EntryRow>,
    #[template_child]
    timeout_sec_entry: TemplateChild<adw::EntryRow>,
    #[template_child]
    lazy_unmount_switch: TemplateChild<adw::SwitchRow>,
    #[template_child]
    sloppy_options_switch: TemplateChild<adw::SwitchRow>,
    #[template_child]
    read_write_only_switch: TemplateChild<adw::SwitchRow>,
    #[template_child]
    force_unmount_switch: TemplateChild<adw::SwitchRow>,

    #[template_child]
    wanted_by_entry: TemplateChild<SuggestionRow>,

    pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,

    pub(super) file_data: RefCell<UnitFileData>,

    file_system_names: RefCell<BTreeSet<String>>,
    resources_to_mount: RefCell<BTreeSet<String>>,

    directory_mode_validator: OnceCell<Regex>,
    directory_mode_typing_validator: OnceCell<Regex>,
}

impl CreatorPageMountImp {
    pub fn advanced_mode(&self, advanced: bool) {
        self.mount_avanced_group.set_visible(advanced);
    }
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

        self.obj().connect_showing(|page| {
            let page = page.imp();
            if !page.file_system_names.borrow().is_empty()
                || !page.resources_to_mount.borrow().is_empty()
            {
                return;
            }

            let page = page.obj().clone();

            glib::spawn_future_local(async move {
                let (file_system_names, resources_to_mount) = runtime().block_on(async move {
                    let h1 = tokio::spawn(async { mount_tools::fetch_filesystem_names().await });
                    let h2 = tokio::spawn(async { mount_tools::fetch_resources_to_mount().await });

                    tokio::join!(h1, h2)
                });

                let file_system_names = match file_system_names {
                    Ok(Ok(set)) => set,
                    Ok(Err(err1)) => {
                        warn!("Fetch File System Names Error {}", err1);
                        BTreeSet::default()
                    }
                    Err(err1) => {
                        warn!("Fetch File System Names Error {}", err1);
                        BTreeSet::default()
                    }
                };

                let resources_to_mount = match resources_to_mount {
                    Ok(Ok(set)) => set,
                    Ok(Err(err2)) => {
                        warn!("Fetch Resources to Mount Error {}", err2);
                        BTreeSet::default()
                    }
                    Err(err2) => {
                        warn!("Fetch Resources to Mount Error {}", err2);
                        BTreeSet::default()
                    }
                };

                let vec: Vec<&str> = file_system_names.iter().map(|s| s.as_str()).collect();
                let string_list = gtk::StringList::new(&vec);

                let page = page.imp();
                page.mount_type_suggestion.set_model(Some(&string_list));

                let vec: Vec<&str> = resources_to_mount.iter().map(|s| s.as_str()).collect();
                let string_list = gtk::StringList::new(&vec);
                page.what_entry.set_model(Some(&string_list));

                page.file_system_names.replace(file_system_names);
                page.resources_to_mount.replace(resources_to_mount);
            });
        });

        let event_focus = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_focus.connect_leave(move |event| {
            if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                let this = upgrade!(this);
                this.validate_directory_mode(entry);
            }
        });
        self.directory_mode_entry.add_controller(event_focus);
        let this = self.downgrade();
        self.directory_mode_entry.connect_changed(move |entry| {
            let this = upgrade!(this);
            this.validate_directory_mode_text_change(entry);
        });
        self.what_entry.set_popup_width(400);

        let event_focus = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_focus.connect_leave(move |_| {
            let this = upgrade!(this);
            this.validate_where();
        });
        self.where_entry.add_controller(event_focus);
        let this = self.downgrade();
        self.where_entry.connect_changed(move |_| {
            let this = upgrade!(this);
            this.validate_where();
        });

        let event_focus = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_focus.connect_leave(move |event| {
            if let Some(entry) = event.widget().and_downcast_ref::<SuggestionRow>() {
                let this = upgrade!(this);
                this.validate_what(entry);
            }
        });
        self.what_entry.add_controller(event_focus);

        let event_controller = widget::clear_on_escape();
        self.description_entry.add_controller(event_controller);

        let event_focus = gtk::EventControllerFocus::new();
        event_focus.connect_leave(|event| {
            if let Some(entry) = event.widget().and_downcast_ref::<adw::EntryRow>() {
                creator::creator_page_timer::validator::validate_monotonic_entry(
                    "TimeoutSec".to_owned(),
                    entry,
                )
            }
        });
        self.timeout_sec_entry.add_controller(event_focus);

        self.wanted_by_entry.set_popup_width(400);
        let event_controller = gtk::EventControllerFocus::new();
        let this = self.downgrade();
        event_controller.connect_leave(move |_event| {
            let this = upgrade!(this);

            this.validate_unit_wanted_by();
        });
        self.wanted_by_entry.add_controller(event_controller);
    }
}

#[gtk::template_callbacks]
impl CreatorPageMountImp {
    #[template_callback]
    fn where_search_dialog_clicked(&self, _button: gtk::Button) {
        let file_dialog = gtk::FileDialog::builder()
            //Title of the folder selection widget window
            .title(pgettext("create_unit", "Select a mount point"))
            //Button title of the folder selection widget
            .accept_label(pgettext("create_unit", "Select"))
            .build();

        let create_service_page = self.obj().clone();

        let text = self.where_entry.text();
        if text.is_empty() {
            let mnt = PathBuf::from("/mnt");
            if mnt.exists() {
                let dir = gio::File::for_path(mnt);
                file_dialog.set_initial_folder(Some(&dir));
            }
        } else {
            let path = Path::new(&text);
            if path.exists() {
                let file = gio::File::for_path(path);
                file_dialog.set_initial_file(Some(&file));
            } else {
                warn!("not exist {}", path.display());
                creator::set_initial_folder(&file_dialog);
            }
        }

        let win = self.window.get().and_then(|w| w.upgrade());
        let win = win.and_upcast_ref::<gtk::Window>();

        file_dialog.select_folder(win, None::<&gio::Cancellable>, move |result| match result {
            Ok(file) => {
                if let Some(path) = file.path() {
                    info!("selected path {}", path.display());
                    let file_path_str = path.display().to_string();
                    create_service_page
                        .imp()
                        .where_entry
                        .set_text(&file_path_str);
                }
            }
            Err(e) => warn!("Unit File Selection Error {e:?}"),
        });
    }

    #[template_callback]
    fn exec_start_dialog_clicked(&self, _button: gtk::Button) {}
}

impl CreatorPageMountImp {
    fn validate_unit_wanted_by(&self) {
        common::validate_unit_common(
            &self.wanted_by_entry.get(),
            "WantedBy",
            Some(&self.wanted_by_entry.text()),
            self.window(),
        );
    }

    fn window(&self) -> &WeakRef<UnitCreatorWindow> {
        self.window.get().unwrap()
    }

    pub(super) fn update_from_unit_info(&self) {
        let window = upgrade_opt!(self.window.get());

        let model = window.imp().get_trigger_units_model();

        let single_selection_model = gtk::SingleSelection::builder()
            .can_unselect(true)
            .autoselect(false)
            .model(&model)
            .build();

        self.wanted_by_entry
            .set_model(Some(&single_selection_model));
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
        file_data.set_what(self.what_entry.text());
        file_data.set_wherex(self.where_entry.text());
        file_data.set_typex(self.mount_type_suggestion.text());
        file_data.set_options(self.mount_options_entry.text());
        file_data.set_directory_mode(self.directory_mode_entry.text());
        file_data.set_timeout_sec(self.timeout_sec_entry.text());
        file_data.set_lazy_unmount(self.lazy_unmount_switch.is_active());
        file_data.set_sloppy_options(self.sloppy_options_switch.is_active());
        file_data.set_read_write_only(self.read_write_only_switch.is_active());
        file_data.set_force_unmount(self.force_unmount_switch.is_active());

        file_data.sort();
    }

    pub fn update_from_file_content(&self, content: &str) {
        debug!("{}", content);
        let Some(data) = UnitFileData::from_content(content) else {
            return;
        };

        debug!("{:#?}", data);

        self.description_entry.set_text(data.description());

        self.what_entry.set_text(data.what());
        self.where_entry.set_text(data.wherex());
        self.mount_type_suggestion.set_text(data.typex());
        self.mount_options_entry.set_text(data.options());
        self.directory_mode_entry.set_text(data.directory_mode());
        self.timeout_sec_entry.set_text(data.timeout_sec());
        self.lazy_unmount_switch.set_active(data.lazy_unmount());
        self.sloppy_options_switch.set_active(data.sloppy_options());
        self.read_write_only_switch
            .set_active(data.read_write_only());
        self.force_unmount_switch.set_active(data.force_unmount());

        self.file_data.replace(data);
    }

    fn directory_mode_validator(&self) -> &Regex {
        self.directory_mode_validator
            .get_or_init(validator::directory_mode_validator)
    }
    fn directory_mode_typing_validator(&self) -> &Regex {
        self.directory_mode_typing_validator
            .get_or_init(validator::directory_mode_typing_validator)
    }

    fn validate_directory_mode(&self, entry: &adw::EntryRow) {
        let text = entry.text();
        let text = text.trim();
        let prefix = DIRECTORYMODE;
        info!("Validating {:?} {:?}", prefix, text);

        const MIN: usize = 3;
        let name_err = if text.is_empty() {
            CreateUnitErr::NoErr
        } else if text.len() < MIN {
            CreateUnitErr::TooShort(MIN)
        } else if text.len() > DMODE_MAX {
            CreateUnitErr::TooLong(DMODE_MAX)
        } else if !self.directory_mode_validator().is_match(text) {
            CreateUnitErr::Malformed
        } else {
            if text.len() == MIN {
                entry.set_text(&format!("0{}", text));
            } else {
                entry.set_text(text);
            }
            CreateUnitErr::NoErr
        };

        CreateUnitErr::apply_validation_result(entry, &name_err, prefix);
    }

    fn validate_directory_mode_text_change(&self, entry: &adw::EntryRow) {
        let text = entry.text();
        let prefix = DIRECTORYMODE;
        info!("Validating {:?} {:?}", prefix, text);

        const MAX: usize = 4;
        let name_err = if text.is_empty() {
            CreateUnitErr::NoErr
        } else if text.len() > MAX {
            CreateUnitErr::TooLong(MAX)
        } else if !self
            .directory_mode_typing_validator()
            .is_match(text.as_str())
        {
            CreateUnitErr::WrongChar
        } else {
            CreateUnitErr::NoErr
        };

        CreateUnitErr::apply_validation_result(entry, &name_err, prefix);
    }

    pub(super) fn validate(&self) -> bool {
        let mut valid = self.validate_what(&self.what_entry);
        valid &= self.validate_where();
        valid
    }

    fn validate_what(&self, entry: &SuggestionRow) -> bool {
        let text = entry.text();
        let prefix = "What";
        info!("Validating {:?} {:?}", prefix, text);

        let name_err = if text.is_empty() {
            CreateUnitErr::Mandatory
        } else {
            CreateUnitErr::NoErr
        };

        CreateUnitErr::apply_validation_result_error(entry, &name_err, prefix, true)
    }

    fn validate_where(&self) -> bool {
        let entry = self.where_entry.get();
        let text = entry.text();
        let prefix = "Where";
        info!("Validating {:?} {:?}", prefix, text);

        let name_err = if text.is_empty() {
            CreateUnitErr::Mandatory
        } else {
            let path = PathBuf::from(&text);
            if !path.is_absolute() {
                CreateUnitErr::NotAbsolute
            } else {
                let escaped_prefix = mount_tools::escape_path(&text);
                self.update_unit_prefix(escaped_prefix)
            }
        };

        CreateUnitErr::apply_validation_result_error(&entry, &name_err, prefix, true)
    }

    fn update_unit_prefix(&self, escaped_prefix: String) -> CreateUnitErr {
        if let Some(window) = self.window.get() {
            let window = upgrade!(window, CreateUnitErr::Unknown);
            window.update_unit_prefix(escaped_prefix)
        } else {
            CreateUnitErr::Unknown
        }
    }
}

impl WidgetImpl for CreatorPageMountImp {}

impl NavigationPageImpl for CreatorPageMountImp {}
