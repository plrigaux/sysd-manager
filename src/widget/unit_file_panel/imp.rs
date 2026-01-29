use std::{
    cell::{Cell, OnceCell, RefCell},
    ffi::OsStr,
    path::Path,
};

use crate::{
    consts::{ADWAITA, APP_ACTION_DAEMON_RELOAD_BUS, SUGGESTED_ACTION},
    format2,
    systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
    systemd_gui::{self, is_dark},
    upgrade,
    utils::font_management::set_text_view_font_display,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::{
            data::{KEY_PREF_UNIT_FILE_LINE_NUMBERS, PREFERENCES},
            style_scheme::style_schemes,
        },
        text_search::{self, on_new_text},
        unit_file_panel::flatpak::PROCEED,
    },
};
use adw::prelude::*;
use base::file::determine_drop_in_path_dir;
use gettextrs::{gettext, pgettext};
use gtk::{
    TemplateChild,
    ffi::GTK_INVALID_LIST_POSITION,
    gio::SimpleAction,
    glib,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
};
use log::{debug, info, warn};
use regex::Regex;
use sourceview5::{Buffer, prelude::*};
use std::fmt::Write;
use systemd::sysdbus::proxy_service_name;
use tokio::sync::oneshot::Receiver;
use tracing::error;

use super::flatpak;

const PANEL_EMPTY: &str = "empty";
const PANEL_FILE: &str = "file_panel";
const DEFAULT_DROP_IN_FILE_NAME: &str = "override";
const UNIT_FILE_ID: &str = "unit_file";
const TEXT_FIND_ACTION: &str = "unit_file_text_find";
const UNIT_FILE_LINE_NUMBER_ACTION: &str = "unit_file_line_number";

#[derive(PartialEq, Copy, Clone)]
enum UnitFileStatus {
    Create,
    Edit,
}

#[derive(Clone)]
struct FileNav {
    file_path: String,
    id: String,
    status: UnitFileStatus,
    is_drop_in: bool,
    is_runtime: bool,
}

impl FileNav {
    fn is_file(&self) -> bool {
        !self.is_drop_in
    }

    fn file_stem(&self) -> Option<&str> {
        Path::new(&self.file_path)
            .file_stem()
            .and_then(OsStr::to_str)
    }
}

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
pub struct UnitFilePanelImp {
    #[template_child]
    save_button: TemplateChild<gtk::Button>,

    unit_file_text: OnceCell<sourceview5::View>,

    sourceview5_buffer: OnceCell<sourceview5::Buffer>,

    #[template_child]
    unit_file_scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    file_link: TemplateChild<gtk::LinkButton>,

    #[template_child]
    panel_file_stack: TemplateChild<adw::ViewStack>,

    #[template_child]
    file_dropin_selector: TemplateChild<adw::ToggleGroup>, //TODOa handle create one

    #[template_child]
    unit_file_menu: TemplateChild<gio::MenuModel>,

    #[template_child]
    text_search_bar: TemplateChild<gtk::SearchBar>,

    #[template_child]
    find_text_button: TemplateChild<gtk::ToggleButton>,

    app_window: OnceCell<AppWindow>,

    visible_on_page: Cell<bool>,

    unit: RefCell<Option<UnitInfo>>,

    unit_dependencies_loaded: Cell<bool>,

    all_unit_files: RefCell<Vec<FileNav>>,

    file_content_selected_index: Cell<u32>,
    //file_displayed: RefCell<Option<String>>,
    //file_status: RefCell<Option<UnitFileStatus>>,
}

macro_rules! get_buffer {
    ($self:expr) => {{
        let buffer = $self
            .unit_file_text
            .get()
            .expect("unit_file_text shall be set")
            .buffer();

        buffer.downcast::<Buffer>().expect("suppose to be Buffer")
    }};
}

#[gtk::template_callbacks]
impl UnitFilePanelImp {
    #[template_callback]
    fn save_file(&self, button: &gtk::Button) {
        debug!("button {button:?}");

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        let buffer = self
            .unit_file_text
            .get()
            .expect("expect sourceview5::View")
            .buffer();

        //create or edit the file

        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true);

        let binding = self.all_unit_files.borrow();
        let Some(file_nav) = binding
            .get(self.file_content_selected_index.get() as usize)
            .cloned()
        else {
            warn!("No file path to save");
            return;
        };

        let file_panel = self.obj().clone();
        let level = unit.dbus_level();
        let unit_name = unit.primary();
        let unit_name2 = unit.primary();
        let user_session = level.user_session();
        if file_nav.status == UnitFileStatus::Create {
            let (cleaned_text, file_stem) = Self::clean_create_text(&unit.primary(), text.as_str());

            let file_stem = if let Some(file_stem) = file_stem {
                file_stem
            } else {
                DEFAULT_DROP_IN_FILE_NAME.to_owned()
            };

            let unique_drop_in_stem = self.unique_drop_in_stem(&file_stem);

            glib::spawn_future_local(async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                systemd::runtime().spawn(async move {
                    let response = systemd::create_drop_in(
                        user_session,
                        file_nav.is_runtime,
                        &unit_name,
                        &unique_drop_in_stem,
                        &cleaned_text,
                    )
                    .await;

                    sender
                        .send(response)
                        .expect("The channel needs to be open.");
                });

                file_panel
                    .imp()
                    .handle_save_response(
                        receiver,
                        file_nav.status,
                        &file_nav.file_path,
                        &unit_name2,
                        user_session,
                    )
                    .await;
            });
        } else {
            let file_path = file_nav.file_path.clone();
            glib::spawn_future_local(async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();

                let content = remove_trailing_newlines(&text)
                    .inspect_err(|e| warn!("{e:?}"))
                    .unwrap_or(text.to_string());

                systemd::runtime().spawn(async move {
                    let response = systemd::save_file(level, &file_path, &content).await;

                    sender
                        .send(response)
                        .expect("The channel needs to be open.");
                });

                file_panel
                    .imp()
                    .handle_save_response(
                        receiver,
                        file_nav.status,
                        &file_nav.file_path,
                        &unit_name2,
                        user_session,
                    )
                    .await;
            });
        }
    }

    async fn handle_save_response<T>(
        &self,
        receiver: Receiver<Result<T, SystemdErrors>>,
        status: UnitFileStatus,
        file_path: &str,
        unit_name: &str,
        user_session: bool,
    ) {
        let (msg, use_mark_up, action) = match receiver.await.expect("Tokio receiver works") {
            Ok(_a) => {
                let msg = match status {
                    UnitFileStatus::Create => pgettext("file", "File {} created successfully!"),
                    UnitFileStatus::Edit => pgettext("file", "File {} saved successfully!"),
                };
                let file_path_format = format!("<u>{}</u>", file_path);
                let msg = format2!(msg, file_path_format);

                // Suggest to reload all unit configuation
                let button_label = gettext("Daemon Reload");
                (
                    msg,
                    true,
                    Some((APP_ACTION_DAEMON_RELOAD_BUS, button_label, user_session)),
                )
            }
            Err(error) => {
                warn!(
                    "Unit {:?}, Unable to save file: {:?}, Error {:?}",
                    unit_name, file_path, error
                );

                match error {
                    SystemdErrors::NotAuthorized => (
                        pgettext("file", "Not able to save file, permission not granted!"),
                        false,
                        None,
                    ),
                    SystemdErrors::ZFdoServiceUnknowm(_s) => {
                        // Service Name
                        // Action Start it or install it
                        let service_name = proxy_service_name();
                        let dialog = flatpak::proxy_service_not_started(service_name.as_deref());
                        let window = self.app_window.get().expect("AppWindow supposed to be set");

                        dialog.present(Some(window));
                        (
                            pgettext("file", "Not able to save file, permission not granted!"),
                            false,
                            None,
                        )
                    }

                    SystemdErrors::CmdNoFreedesktopFlatpakPermission(_, _) => {
                        let dialog = flatpak::flatpak_permision_alert();
                        dialog.present(self.app_window.get());
                        (
                            pgettext(
                                "file",
                                "Not able to save file, Flatpak permission not granted!",
                            ),
                            false,
                            None,
                        )
                    }

                    _ => (
                        pgettext("file", "Not able to save file, an error happened!"),
                        false,
                        None,
                    ),
                }
            }
        };

        self.add_toast_message(&msg, use_mark_up, action);
    }
}

macro_rules! get_unit {
    ($self:expr) => {{
        let binding = $self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("No unit to present");
            $self.set_editor_text("", false);
            return;
        };
        unit.clone()
    }};
}

impl UnitFilePanelImp {
    fn clean_create_text(unit_name: &str, text: &str) -> (String, Option<String>) {
        let mut cleaned_text = String::new();

        let re_str = format!(r"/(run|etc)/systemd/system/{}.d/(.+).conf$", unit_name);
        let re = Regex::new(&re_str).expect("Valid RegEx");
        let mut content = false;
        let mut file_name: Option<_> = None;

        for line in text.lines() {
            if !line.starts_with("###") {
                content = true;

                let trimmed_line = line.trim_end();
                cleaned_text.push_str(trimmed_line);

                if content {
                    // New section starts, add a newline before it
                    writeln!(cleaned_text).expect("Writing to string should work");
                }
            } else if content {
                break;
            } else if let Some(caps) = re.captures(line) {
                file_name = Some(caps[2].to_string());
            }
        }

        cleaned_text = remove_trailing_newlines(&cleaned_text)
            .inspect_err(|e| warn!("{e:?}"))
            .unwrap_or(cleaned_text);

        (cleaned_text, file_name)
    }

    fn add_toast_message(&self, message: &str, markup: bool, action: Option<(&str, String, bool)>) {
        if let Some(app_window) = self.app_window.get() {
            app_window.add_toast_message(message, markup, action);
        }
    }

    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        if self.visible_on_page.get()
            && !self.unit_dependencies_loaded.get()
            && self.unit.borrow().is_some()
        {
            self.set_file_content_init()
        }
    }

    fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                self.file_content_selected_index.set(0);
                self.unit.replace(None);
                self.set_file_content_init();
                return;
            }
        };

        let old_unit = self.unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit
            && old_unit.primary() != unit.primary()
        {
            self.unit_dependencies_loaded.set(false)
        }

        self.file_content_selected_index.set(0);
        self.set_file_content_init()
    }

    pub fn set_file_content_init(&self) {
        if !self.visible_on_page.get() {
            return;
        }

        let unit = get_unit!(self);

        let object_path = unit.object_path();
        let level = unit.dbus_level();

        let unit_file_panel = self.obj().clone();
        glib::spawn_future_local(async move {
            let (sender, receiver) = tokio::sync::oneshot::channel();

            crate::systemd::runtime().spawn(async move {
                let response = systemd::fetch_drop_in_paths(level, &object_path).await;

                sender
                    .send(response)
                    .expect("The channel needs to be open.")
            });

            match receiver.await.expect("Tokio receiver to work well") {
                Ok(drop_in_files) => {
                    unit_file_panel.imp().set_dropins(&drop_in_files);
                }
                Err(err) => {
                    warn!("Fail to update Unit info {err:?}");
                }
            };
        });

        let primary = unit.primary();

        self.set_dropins(&[]);
        self.display_unit_file_content(None, &primary);
    }

    fn display_unit_file_content(&self, file_nav: Option<&FileNav>, primary: &str) {
        match file_nav {
            Some(file_nav) => {
                self.display_unit_file_content2(primary, file_nav);
            }
            None => {
                let all_files = self.all_unit_files.borrow();

                if all_files.is_empty() {
                    self.fill_gui_content(String::new(), false, "");
                    return;
                }

                let file_nav = all_files.first().expect("vector should not be empty");

                self.display_unit_file_content2(primary, file_nav);
            }
        };
    }

    fn display_unit_file_content2(&self, primary: &str, file_nav: &FileNav) {
        let (file_content, is_error_msg) =
            systemd::get_unit_file_info(Some(&file_nav.file_path), primary)
                .map(|content| (content, false))
                .unwrap_or_else(|e| {
                    warn!("get_unit_file_info Error: {e:?}");

                    #[cfg(feature = "flatpak")]
                    {
                        let mut body = String::new();
                        body.push_str("You miss a permission to be able to read the file.\n\n");
                        body.push_str(
                            "To know how to acquire needed permissions, follow this link:\n\n\
                         https://github.com/plrigaux/sysd-manager/wiki/Flatpak",
                        );
                        (body, true)
                    }

                    #[cfg(not(feature = "flatpak"))]
                    (String::new(), true)
                });

        self.fill_gui_content(file_content, is_error_msg, &file_nav.file_path);
    }

    fn fill_gui_content(&self, file_content: String, is_error_msg: bool, file_path: &str) {
        let uri = generate_file_uri(file_path);

        self.file_link.set_uri(&uri);

        self.file_link.set_label(file_path);

        self.set_editor_text(&file_content, is_error_msg);
    }

    fn display_unit_drop_in_file_content(&self, drop_in_index: u32) {
        let binding = self.all_unit_files.borrow();
        let Some(file_nav) = binding.get(drop_in_index as usize) else {
            warn!(
                "Drop in index out of bound requested: {drop_in_index} max: {}",
                self.all_unit_files.borrow().len()
            );
            self.set_editor_text("", false);
            return;
        };

        let unit = get_unit!(self);
        let primary = unit.primary();
        self.display_unit_file_content(Some(file_nav), &primary);
    }

    fn set_dropins(&self, drop_in_files: &[String]) {
        {
            let mut all_files = self.all_unit_files.borrow_mut();
            all_files.clear();

            if let Some(file_path) = get_unit!(self).file_path() {
                let fnav = FileNav {
                    file_path,
                    id: UNIT_FILE_ID.to_string(),
                    status: UnitFileStatus::Edit,
                    is_drop_in: false,
                    is_runtime: false, //TODO find out real status
                };
                all_files.push(fnav);
            }

            for (idx, drop_in_file) in drop_in_files.iter().enumerate() {
                let name = format!("dropin {idx}");

                let fnav = FileNav {
                    file_path: drop_in_file.clone(),
                    id: name,
                    status: UnitFileStatus::Edit,
                    is_drop_in: true,
                    is_runtime: false, //TODO find out real status
                };
                all_files.push(fnav);
            }
        }
        self.set_drop_ins_selector();
    }

    fn set_drop_ins_selector(&self) {
        self.file_dropin_selector.remove_all();
        let all_files = self.all_unit_files.borrow();
        let all_files_len = all_files.len();
        let mut idx = 1;

        for file_nav in all_files.iter() {
            let label_text = if file_nav.is_file() {
                pgettext("file", "Unit File")
            } else {
                let label_text = pgettext("file", "Drop In");

                if all_files_len > 2 {
                    let label = format!("{label_text} {idx}");
                    idx += 1;
                    label
                } else {
                    label_text
                }
            };

            let toggle = adw::Toggle::builder()
                .label(&label_text)
                .name(&file_nav.id)
                .tooltip(file_nav.file_path.clone())
                .build();
            self.file_dropin_selector.add(toggle);
        }

        let visible = all_files_len > 1;

        self.file_dropin_selector.set_visible(visible);

        self.set_visible_child_panel();
    }

    fn file_dropin_selector_activate(&self, selected_index: u32) {
        if self.file_content_selected_index.get() == selected_index {
            return;
        }

        self.file_content_selected_index.set(selected_index);

        self.display_unit_drop_in_file_content(selected_index);
    }

    fn set_editor_text(&self, file_content: &str, is_error_msg: bool) {
        let view = self.unit_file_text.get().expect("expect sourceview5::View");
        let buf = view.buffer();

        if let Some(buffer) = buf.downcast_ref::<Buffer>() {
            if is_error_msg {
                buffer.set_language(None);
            } else if buffer.language().is_none()
                && let Some(ref language) = sourceview5::LanguageManager::new().language("ini")
            {
                buffer.set_language(Some(language));
            }
        }

        buf.set_text(""); //To clear current
        buf.set_text(file_content);

        self.save_button.set_sensitive(false);
        self.set_visible_child_panel();

        on_new_text(&self.text_search_bar);
    }

    fn set_visible_child_panel(&self) {
        let panel = if self.all_unit_files.borrow().is_empty() {
            PANEL_EMPTY
        } else {
            PANEL_FILE
        };

        self.panel_file_stack.set_visible_child_name(panel);
    }

    fn set_dark(&self, is_dark: bool) {
        let style_scheme_id = PREFERENCES.unit_file_style_scheme();

        debug!("File Unit set_dark {is_dark} style_scheme_id {style_scheme_id:?}");

        self.set_new_style_scheme(Some(&style_scheme_id));
    }

    fn set_new_style_scheme(&self, style_scheme_id: Option<&str>) {
        info!("Set new style scheme {style_scheme_id:?}");

        match style_scheme_id {
            Some("") | None => {
                let buffer = get_buffer!(self);

                buffer.set_style_scheme(None);
            }
            Some(style_scheme_id) => {
                let style_schemes_map: &'static std::collections::BTreeMap<
                    String,
                    crate::widget::preferences::style_scheme::StyleSchemes,
                > = style_schemes();

                debug!("{style_schemes_map:#?}");
                /*             if style_scheme_id.is_empty() {
                    style_scheme_id = ADWAITA;
                } */

                let style_scheme_st = style_schemes_map.get(style_scheme_id);

                let style_sheme_st = match style_scheme_st {
                    Some(ss) => ss,
                    None => {
                        info!(
                            "Style scheme id \"{style_scheme_id}\" not found in {:?}",
                            style_schemes_map.keys().collect::<Vec<_>>()
                        );

                        //fallback on style Adwaita
                        if let Some(style_scheme_st) = style_schemes_map.get(ADWAITA) {
                            style_scheme_st
                        } else
                        //fallback on first item
                        if let Some((_, style_scheme_st)) =
                            style_schemes_map.first_key_value()
                        {
                            style_scheme_st
                        } else {
                            return;
                        }
                    }
                };

                let scheme_id = &style_sheme_st.get_style_scheme_id(is_dark());

                if let Some(ref scheme) = sourceview5::StyleSchemeManager::new().scheme(scheme_id) {
                    let buffer = get_buffer!(self);
                    info!("Style Scheme found for id {scheme_id:?}");
                    buffer.set_style_scheme(Some(scheme));
                } else {
                    warn!("No Style Scheme found for id {scheme_id:?}")
                }
            }
        }
    }

    pub(crate) fn register(&self, app_window: &AppWindow) {
        if let Err(err) = self.app_window.set(app_window.clone()) {
            error!("Error {:?}", err);
            return;
        }

        let rename_drop_in_file = gio::ActionEntry::builder("rename_drop_in_file")
            .activate(move |_application: &AppWindow, _b, _target_value| {
                info!("call rename_drop_in_file");
            })
            .build();

        let create_drop_in_file_runtime = {
            let unit_file_panel = self.obj().clone();
            gio::ActionEntry::builder("create_drop_in_file_runtime")
                .activate(
                    move |_application: &AppWindow, _b: &SimpleAction, _target_value| {
                        info!("call create_drop_in_file_runtime");
                        let _ = unit_file_panel
                            .imp()
                            .create_drop_in_file(true)
                            .inspect_err(|e| warn!("{e:?}"));
                    },
                )
                .build()
        };

        let create_drop_in_file_permanent = {
            let unit_file_panel = self.obj().clone();
            gio::ActionEntry::builder("create_drop_in_file_permanent")
                .activate(
                    move |_application: &AppWindow, _b: &SimpleAction, _target_value| {
                        info!("call create_drop_in_file_permanent");
                        let _ = unit_file_panel
                            .imp()
                            .create_drop_in_file(false)
                            .inspect_err(|e| warn!("{e:?}"));
                    },
                )
                .build()
        };

        let revert_unit_file_full = {
            let unit_file_panel = self.obj().clone();
            gio::ActionEntry::builder("revert_unit_file_full")
                .activate(
                    move |_application: &AppWindow, _b: &SimpleAction, _target_value| {
                        info!("call revert_unit_file_full");
                        let _ = unit_file_panel
                            .imp()
                            .revert_unit_file_full()
                            .inspect_err(|e| warn!("{e:?}"));
                    },
                )
                .build()
        };

        let unit_file_line_number = {
            let unit_file_text = self.unit_file_text.get().expect("Need to be set").clone();
            gio::ActionEntry::builder(UNIT_FILE_LINE_NUMBER_ACTION)
                .activate(
                    move |_application: &AppWindow, action: &SimpleAction, _target_value| {
                        if let Some(variant) = action.state()
                            && let Some(show_line_number) = variant.get::<bool>()
                        {
                            let show_line_number = !show_line_number;
                            unit_file_text.set_show_line_numbers(show_line_number);
                            action.set_state(&show_line_number.to_variant());
                        }
                    },
                )
                .state(true.to_variant())
                .parameter_type(Some(glib::VariantTy::BOOLEAN))
                .build()
        };

        let text_search_bar_action_entry =
            text_search::create_action_entry(&self.text_search_bar, TEXT_FIND_ACTION);

        app_window.add_action_entries([
            rename_drop_in_file,
            create_drop_in_file_runtime,
            create_drop_in_file_permanent,
            revert_unit_file_full,
            unit_file_line_number,
            text_search_bar_action_entry,
        ]);
    }

    pub(super) fn refresh_panels(&self) {
        if self.visible_on_page.get() {
            self.set_file_content_init()
        }
    }

    fn create_drop_in_file(&self, runtime: bool) -> Result<(), SystemdErrors> {
        info!("create_drop_in_file called runtime {runtime}");

        //get the file content
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return Ok(());
        };

        let file_path = unit.file_path();
        let primary = unit.primary();
        let file_content = systemd::get_unit_file_info(file_path.as_deref(), &primary)
            .unwrap_or_else(|e| {
                warn!("get_unit_file_info Error: {e:?}");
                "".to_owned()
            });

        let user_session = unit.dbus_level().user_session();

        let drop_in_file_path = self.create_drop_in_file_path(&primary, runtime, user_session)?;

        self.create_drop_in_nav(&drop_in_file_path, runtime);

        self.set_drop_ins_selector();
        self.file_dropin_selector
            .set_active(self.file_dropin_selector.n_toggles() - 1);

        let new_file_content: String = self
            .set_dropin_file_format(file_path, file_content, &drop_in_file_path)
            .inspect_err(|e| warn!("some error {:?}", e))
            .unwrap_or_default();

        self.fill_gui_content(new_file_content, false, &drop_in_file_path);
        Ok(())
    }

    fn create_drop_in_nav(&self, drop_in_file_path: &str, runtime: bool) {
        let fnav = FileNav {
            file_path: drop_in_file_path.to_string(),
            id: "create drop".to_string(),
            status: UnitFileStatus::Create,
            is_drop_in: true,
            is_runtime: runtime,
        };

        self.all_unit_files.borrow_mut().push(fnav);
    }

    fn create_drop_in_file_path(
        &self,
        primary: &str,
        runtime: bool,
        user_session: bool,
    ) -> Result<String, SystemdErrors> {
        let mut path_dir = determine_drop_in_path_dir(primary, runtime, user_session)?;

        let drop_in_stem = self.unique_drop_in_stem(DEFAULT_DROP_IN_FILE_NAME);
        path_dir.push('/');
        path_dir.push_str(&drop_in_stem);
        path_dir.push_str(".conf");

        Ok(path_dir)
    }

    fn unique_drop_in_stem(&self, file_stem: &str) -> String {
        let all_unit_files = self.all_unit_files.borrow();

        let (file_stem, mut idx) = Self::grab_index(file_stem);

        loop {
            let file_stem = if idx == 0 {
                file_stem.to_string()
            } else {
                format!("{}-{}", file_stem, idx)
            };

            if all_unit_files.iter().any(|f| {
                f.is_drop_in
                    && f.status != UnitFileStatus::Create
                    && f.file_stem() == Some(&file_stem)
            }) {
                idx += 1;
                continue;
            }
            return file_stem;
        }
    }

    fn grab_index(file_stem: &str) -> (&str, u32) {
        let re = Regex::new(r"-(\d+)$").expect("Valid RegEx");

        if let Some(caps) = re.captures(file_stem) {
            let start = caps.get_match().start();

            if let Ok(num) = caps[1].parse::<u32>() {
                return (&file_stem[0..start], num + 1);
            }
        }
        (file_stem, 0)
    }

    fn set_dropin_file_format(
        &self,
        file_path: Option<String>,

        file_content: String,
        drop_in_file_path: &str,
    ) -> Result<String, SystemdErrors> {
        let mut new_file_content = String::with_capacity(file_content.len() * 2);

        writeln!(
            new_file_content,
            "### {} {}",
            // Create Drop in file name
            pgettext("file", "Editing"),
            drop_in_file_path
        )?;

        writeln!(
            new_file_content,
            "### {}",
            // Create Drop in file name
            pgettext("file", "Note: you can change the file name")
        )?;

        writeln!(
            new_file_content,
            "###\n### {}",
            // Create Drop in description
            pgettext(
                "file",
                "Anything between here and the comment below will become the contents of the drop-in file"
            )
        )?;

        new_file_content.push_str("\n\n\n");
        writeln!(
            new_file_content,
            "### {}",
            // Create Drop in file footer
            pgettext("file", "Edits below this comment will be discarded")
        )?;
        new_file_content.push('\n');
        writeln!(new_file_content, "### {}", file_path.unwrap_or_default())?;
        for line in file_content.lines() {
            new_file_content.push_str("# ");
            new_file_content.push_str(line);
            new_file_content.push('\n');
        }

        Ok(new_file_content)
    }

    pub(super) fn set_inter_message(&self, action: &InterPanelMessage) {
        match *action {
            InterPanelMessage::FontProvider(old, new) => {
                let view = self.unit_file_text.get().expect("expect sourceview5::View");

                set_text_view_font_display(old, new, &view.display())
            }
            InterPanelMessage::IsDark(is_dark) => self.set_dark(is_dark),
            InterPanelMessage::PanelVisible(visible) => self.set_visible_on_page(visible),
            InterPanelMessage::NewStyleScheme(style_scheme) => {
                self.set_new_style_scheme(style_scheme)
            }
            InterPanelMessage::UnitChange(unit) => self.set_unit(unit),
            _ => {}
        }
    }

    fn revert_unit_file_full(&self) -> Result<(), SystemdErrors> {
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            return Err(SystemdErrors::NoUnit);
        };
        let unit_name = unit.primary();
        let file_panel = self.obj().clone();
        let dialog = flatpak::revert_drop_in_alert(&unit_name);
        dialog.connect_response(None, move |_dialog, response| {
            info!("Response {response}");

            if response == PROCEED {
                let _ = file_panel.imp().revert_unit_file_full_action();
            }
        });

        let window = self.app_window.get().expect("AppWindow supposed to be set");

        dialog.present(Some(window));

        Ok(())
    }

    fn revert_unit_file_full_action(&self) -> Result<(), SystemdErrors> {
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            return Err(SystemdErrors::NoUnit);
        };

        let file_panel = self.obj().clone();
        let level = unit.dbus_level();
        let unit_name = unit.primary();

        glib::spawn_future_local(async move {
            let unit_name2 = unit_name.clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            systemd::runtime().spawn(async move {
                let response = systemd::revert_unit_file_full(level, &unit_name).await;

                info!("revert_unit_file_full results {:?}", response);

                sender
                    .send(response)
                    .expect("The channel needs to be open.");
            });

            let (msg, use_mark_up, action) = match receiver.await.expect("Tokio receiver works") {
                Ok(_a) => {
                    let msg = pgettext("file", "Unit {} reverted successfully!");
                    let file_path_format = format!("<unit>{}</unit>", unit_name2);
                    let msg = format2!(msg, file_path_format);

                    //file_panel.imp().set_file_content_init(); //because it need relaod

                    //suposed to have no drop-ins
                    file_panel.imp().set_dropins(&[]);

                    // Suggest to reload all unit configuation
                    let button_label = gettext("Daemon Reload");
                    (
                        msg,
                        true,
                        Some((
                            APP_ACTION_DAEMON_RELOAD_BUS,
                            button_label,
                            level.user_session(),
                        )),
                    ) //TODO translate
                }
                Err(error) => {
                    warn!("Unit {:?}, Unable to revert {:?}", unit_name2, error);

                    match error {
                        SystemdErrors::NotAuthorized => (
                            pgettext("file", "Not able to save file, permission not granted!"),
                            false,
                            None,
                        ),
                        SystemdErrors::ZFdoServiceUnknowm(_s) => {
                            // Service Name
                            // Action Start it or install it
                            let service_name = proxy_service_name();
                            let dialog =
                                flatpak::proxy_service_not_started(service_name.as_deref());
                            let window = file_panel
                                .imp()
                                .app_window
                                .get()
                                .expect("AppWindow supposed to be set");

                            dialog.present(Some(window));
                            (
                                pgettext(
                                    "file",
                                    "Not able to reverted unit, permission not granted!",
                                ),
                                false,
                                None,
                            )
                        }

                        _ => (
                            pgettext("file", "Not able to reverted unit, an error happened!"),
                            false,
                            None,
                        ),
                    }
                }
            };

            file_panel
                .imp()
                .add_toast_message(&msg, use_mark_up, action);
        });
        Ok(())
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitFilePanelImp {
    const NAME: &'static str = "UnitFilePanel";
    type Type = super::UnitFilePanel;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();

        klass.install_action("test_pizza", None, |a, b, c| {
            println!("test a {:?} b {:?} c {:?}", a, b, c)
        });
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UnitFilePanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_visible_child_panel();

        let buffer = sourceview5::Buffer::new(None);

        if let Some(ref language) = sourceview5::LanguageManager::new().language("ini") {
            buffer.set_language(Some(language));
        }

        let view = sourceview5::View::with_buffer(&buffer);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(true);
        view.set_tab_width(4);
        view.set_monospace(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);

        self.unit_file_scrolled_window.set_child(Some(&view));

        self.save_button.add_css_class(SUGGESTED_ACTION);
        self.save_button.set_sensitive(false);
        {
            let buffer = view.buffer();

            let save_button = self.save_button.downgrade();
            let unit_file_panel = self.obj().downgrade();
            buffer.connect_end_user_action(move |_buf| {
                let save_button = upgrade!(save_button);

                let unit_file_panel = upgrade!(unit_file_panel);

                let allow_save_condition =
                    !unit_file_panel.imp().all_unit_files.borrow().is_empty(); //TODO check is the text has really changed
                save_button.set_sensitive(allow_save_condition);
            });
        }

        let file_text_view = view.upcast_ref::<gtk::TextView>();
        text_search::text_search_construct(
            file_text_view,
            &self.text_search_bar,
            &self.find_text_button,
            TEXT_FIND_ACTION,
            false,
        );

        let settings = systemd_gui::new_settings();

        let show_line_numbers = settings.boolean(KEY_PREF_UNIT_FILE_LINE_NUMBERS);

        settings
            .bind(KEY_PREF_UNIT_FILE_LINE_NUMBERS, &view, "show-line-numbers")
            .build();

        let ts_item = text_search::create_menu_item(TEXT_FIND_ACTION, &self.text_search_bar);
        let menu = gio::Menu::new();

        // Show Line Number Menu Item
        let menu_label = pgettext("file", "Display Line Numbers");

        let mut action_name = String::from("win.");
        action_name.push_str(UNIT_FILE_LINE_NUMBER_ACTION);

        let mi = gio::MenuItem::new(Some(&menu_label), None);
        mi.set_action_and_target_value(Some(&action_name), Some(&show_line_numbers.to_variant()));

        menu.append_item(&mi);
        menu.append_item(&ts_item);

        let menu_sec = gio::Menu::new();
        menu_sec.append_section(None, &menu);

        file_text_view.set_extra_menu(Some(&menu_sec));

        self.sourceview5_buffer
            .set(buffer)
            .expect("sourceview5_buffer set once");
        self.unit_file_text
            .set(view)
            .expect("unit_file_text set once");
        self.file_dropin_selector.connect_n_toggles_notify(|tg| {
            let selected = tg.active();
            debug!("selected file {selected}");
        });

        let unit_file_panel = self.obj().clone();
        self.file_dropin_selector.connect_active_notify(move |tg| {
            let selected = tg.active();
            if selected == GTK_INVALID_LIST_POSITION {
                return;
            }

            debug!("unit file or drop in: {selected}");
            unit_file_panel
                .imp()
                .file_dropin_selector_activate(selected)
        });
    }
}

impl WidgetImpl for UnitFilePanelImp {}
impl BoxImpl for UnitFilePanelImp {}

fn remove_trailing_newlines(text: &str) -> Result<String, regex::Error> {
    let re = Regex::new(r"[\n]+$")?;
    Ok(re.replace(text, "\n").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grab_index_with_no_suffix() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override");
        assert_eq!(stem, "override");
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_grab_index_with_single_digit() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-1");
        assert_eq!(stem, "override");
        assert_eq!(idx, 2);
    }

    #[test]
    fn test_grab_index_with_multiple_digits() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-42");
        assert_eq!(stem, "override");
        assert_eq!(idx, 43);
    }

    #[test]
    fn test_grab_index_with_multiple_hyphens() {
        let (stem, idx) = UnitFilePanelImp::grab_index("my-override-5");
        assert_eq!(stem, "my-override");
        assert_eq!(idx, 6);
    }

    #[test]
    fn test_grab_index_with_trailing_hyphen() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-");
        assert_eq!(stem, "override-");
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_grab_index_with_non_numeric_suffix() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-abc");
        assert_eq!(stem, "override-abc");
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_grab_index_with_zero() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-0");
        assert_eq!(stem, "override");
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_grab_index_with_large_number() {
        let (stem, idx) = UnitFilePanelImp::grab_index("override-999");
        assert_eq!(stem, "override");
        assert_eq!(idx, 1000);
    }

    #[test]
    fn test_file_nav_is_file() {
        let fnav = FileNav {
            file_path: "/etc/systemd/system/test.service".to_string(),
            id: "unit file".to_string(),
            status: UnitFileStatus::Edit,
            is_drop_in: false,
            is_runtime: false,
        };
        assert!(fnav.is_file());
    }

    #[test]
    fn test_file_nav_is_drop_in() {
        let fnav = FileNav {
            file_path: "/etc/systemd/system/test.service.d/override.conf".to_string(),
            id: "dropin 1".to_string(),
            status: UnitFileStatus::Edit,
            is_drop_in: true,
            is_runtime: false,
        };
        assert!(!fnav.is_file());
    }

    #[test]
    fn test_file_nav_file_stem_regular_file() {
        let fnav = FileNav {
            file_path: "/etc/systemd/system/test.service".to_string(),
            id: "unit file".to_string(),
            status: UnitFileStatus::Edit,
            is_drop_in: false,
            is_runtime: false,
        };
        assert_eq!(fnav.file_stem(), Some("test"));
    }

    #[test]
    fn test_file_nav_file_stem_drop_in() {
        let fnav = FileNav {
            file_path: "/etc/systemd/system/test.service.d/override.conf".to_string(),
            id: "dropin 1".to_string(),
            status: UnitFileStatus::Edit,
            is_drop_in: true,
            is_runtime: false,
        };
        assert_eq!(fnav.file_stem(), Some("override"));
    }

    #[test]
    fn test_file_nav_file_stem_no_extension() {
        let fnav = FileNav {
            file_path: "/etc/systemd/system/test".to_string(),
            id: "test".to_string(),
            status: UnitFileStatus::Edit,
            is_drop_in: false,
            is_runtime: false,
        };
        assert_eq!(fnav.file_stem(), Some("test"));
    }

    #[test]
    fn test_remove_trailing_newlines() {
        let text = "line one\n\nline two\n\n\n";
        let cleaned: String = remove_trailing_newlines(text).unwrap();
        println!("{cleaned}");
        assert_eq!(cleaned, "line one\n\nline two\n");
    }
}
