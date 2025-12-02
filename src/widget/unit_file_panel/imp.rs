use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::AdwDialogExt;
use gettextrs::pgettext;
use gtk::{
    TemplateChild,
    ffi::GTK_INVALID_LIST_POSITION,
    gio::SimpleAction,
    glib,
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
};

use crate::{
    consts::{ADWAITA, SUGGESTED_ACTION},
    format2,
    systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
    upgrade,
    utils::font_management::set_text_view_font_display,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::{data::PREFERENCES, style_scheme::style_schemes},
    },
};
use log::{debug, info, warn};
use sourceview5::{Buffer, prelude::*};
use std::fmt::Write;

use super::flatpak;

const PANEL_EMPTY: &str = "empty";
const PANEL_FILE: &str = "file_panel";

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
    file_dropin_selector: TemplateChild<adw::ToggleGroup>,

    #[template_child]
    unit_file_menu: TemplateChild<gio::MenuModel>,

    app_window: OnceCell<AppWindow>,

    visible_on_page: Cell<bool>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,

    drop_in_files: RefCell<Vec<String>>,

    file_content_selected_index: Cell<u32>,

    file_displayed: RefCell<Option<String>>,
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
        info!("button {button:?}");

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
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let text = buffer.text(&start, &end, true);

        let file_path = self.file_displayed.borrow();

        let Some(file_path) = file_path.as_deref() else {
            warn!("No file path to save");
            return;
        };

        match systemd::save_text_to_file(file_path, &text) {
            Ok((file_path, _bytes_written)) => {
                button.remove_css_class(SUGGESTED_ACTION);

                //File saving success message
                let msg = pgettext("file", "File {} saved successfully!");
                let file_path_format = format!("<u>{file_path}</u>");
                let msg = format2!(msg, file_path_format);

                self.add_toast_message(&msg, true);
            }
            Err(error) => {
                warn!(
                    "Unit {:?}, Unable to save file: {:?}, Error {:?}",
                    unit.primary(),
                    unit.file_path(),
                    error
                );

                match error {
                    SystemdErrors::CmdNoFreedesktopFlatpakPermission(command_line, file_path) => {
                        let dialog = flatpak::new(command_line, file_path);
                        let window = self.app_window.get().expect("AppWindow supposed to be set");

                        dialog.present(Some(window));
                    }

                    SystemdErrors::NotAuthorized => {
                        self.add_toast_message(
                            "Not able to save file, permission not granted!",
                            false,
                        );
                    }
                    _ => {
                        self.add_toast_message("Not able to save file, an error happened!", false);
                    }
                }
            }
        };
    }
}

macro_rules! get_unit {
    ($self:expr) => {{
        let binding = $self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("No unit to present");
            $self.set_editor_text("");
            return;
        };
        unit.clone()
    }};
}
impl UnitFilePanelImp {
    fn add_toast_message(&self, message: &str, markup: bool) {
        if let Some(app_window) = self.app_window.get() {
            app_window.add_toast_message(message, markup);
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

            match receiver.await.expect("Tokio receiver works") {
                Ok(drop_in_files) => {
                    unit_file_panel.imp().drop_in_files.replace(drop_in_files);
                    unit_file_panel.imp().set_dropins();
                }
                Err(err) => {
                    warn!("Fail to update Unit info {err:?}");
                }
            };
        });

        let file_path = unit.file_path();
        let primary = unit.primary();
        self.display_unit_file_content(file_path, &primary);
    }

    fn display_unit_file_content(&self, file_path: Option<String>, primary: &str) {
        self.file_displayed.replace(file_path);

        let binding = self.file_displayed.borrow();
        let file_path = binding.as_deref();
        let file_content = systemd::get_unit_file_info(file_path, primary).unwrap_or_else(|e| {
            warn!("get_unit_file_info Error: {e:?}");
            "".to_owned()
        });

        let file_path = file_path.unwrap_or_default();

        let uri = generate_file_uri(file_path);

        self.file_link.set_uri(&uri);

        self.file_link.set_label(file_path);

        self.set_editor_text(&file_content);
    }

    fn display_unit_drop_in_file_content(&self, drop_in_index: u32) {
        let drop_in_files = self.drop_in_files.borrow();

        let Some(file_path) = drop_in_files.get(drop_in_index as usize) else {
            warn!(
                "Drop in index out of bound requested: {drop_in_index} max: {}",
                drop_in_files.len()
            );
            self.set_editor_text("");
            return;
        };

        let unit = get_unit!(self);
        let primary = unit.primary();
        self.display_unit_file_content(Some(file_path.to_owned()), &primary);
    }

    fn set_dropins(&self) {
        let drop_in_files = self.drop_in_files.borrow();
        self.file_dropin_selector.remove_all();

        if drop_in_files.is_empty() {
            self.file_dropin_selector.set_visible(false);
            return;
        }

        self.file_dropin_selector.set_visible(true);

        //Unit file / Drop-Ins selector
        let label_text = pgettext("file", "Unit File");

        let toggle = adw::Toggle::builder()
            .label(label_text)
            .name("file")
            .build();
        self.file_dropin_selector.add(toggle);

        for (idx, drop_in_file) in drop_in_files.iter().enumerate() {
            //Unit file / Drop-Ins selector
            let label_text = pgettext("file", "Drop In");

            let label = if drop_in_files.len() > 1 {
                format!("{label_text} {idx}")
            } else {
                label_text
            };

            let toggle = adw::Toggle::builder()
                .label(label)
                .name(format!("dropin {idx}"))
                .tooltip(drop_in_file)
                .build();
            self.file_dropin_selector.add(toggle);
        }

        self.set_visible_child_panel();
    }

    fn file_dropin_selector_activate(&self, selected_index: u32) {
        if self.file_content_selected_index.get() == selected_index {
            return;
        }

        self.file_content_selected_index.set(selected_index);

        if selected_index == 0 {
            let unit = get_unit!(self);
            self.display_unit_file_content(unit.file_path(), &unit.primary());
        } else {
            self.display_unit_drop_in_file_content(selected_index - 1);
        }
    }

    fn set_editor_text(&self, file_content: &str) {
        let buf = self
            .unit_file_text
            .get()
            .expect("expect sourceview5::View")
            .buffer();

        buf.set_text(""); //To clear current
        buf.set_text(file_content);

        self.save_button.set_sensitive(false);
        self.set_visible_child_panel();
    }

    fn set_visible_child_panel(&self) {
        let file_path = self.unit.borrow().as_ref().and_then(|u| u.file_path());

        let panel = if file_path.is_none() && self.drop_in_files.borrow().is_empty() {
            PANEL_EMPTY
        } else {
            PANEL_FILE
        };

        self.panel_file_stack.set_visible_child_name(panel);
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);

        let style_scheme_id = PREFERENCES.unit_file_style_scheme();

        debug!("File Unit set_dark {is_dark} style_scheme_id {style_scheme_id:?}");

        self.set_new_style_scheme(Some(&style_scheme_id));
    }

    fn set_line_number(&self, line_number: bool) {
        if let Some(view) = self.unit_file_text.get() {
            view.set_show_line_numbers(line_number);
        }
    }

    fn set_new_style_scheme(&self, style_scheme_id: Option<&str>) {
        /*         if !PREFERENCES.unit_file_line_number() {
            style_scheme_id = None
        } */

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

                let scheme_id = &style_sheme_st.get_style_scheme_id(self.is_dark.get());

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
        if self.app_window.set(app_window.clone()).is_ok() {
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
                            _b.is_enabled();
                            unit_file_panel.imp().create_drop_in_file(true);
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
                            _b.is_enabled();
                            unit_file_panel.imp().create_drop_in_file(false);
                        },
                    )
                    .build()
            };

            let revert_drop_in_file_only = gio::ActionEntry::builder("revert_drop_in_file_only")
                .activate(
                    move |_application: &AppWindow, _b: &SimpleAction, _target_value| {
                        info!("call revert_drop_in_file_only");
                        _b.is_enabled();
                    },
                )
                .build();

            let revert_unit_file_full = gio::ActionEntry::builder("revert_unit_file_full")
                .activate(
                    move |_application: &AppWindow, _b: &SimpleAction, _target_value| {
                        info!("call revert_unit_file_full");
                        _b.is_enabled();
                    },
                )
                .build();

            app_window.add_action_entries([
                rename_drop_in_file,
                create_drop_in_file_runtime,
                create_drop_in_file_permanent,
                revert_drop_in_file_only,
                revert_unit_file_full,
            ]);

            if let Some(action) = app_window
                .lookup_action("create_drop_in_file_runtime")
                .and_downcast_ref::<gio::SimpleAction>()
            {
                let b = action.is_enabled();
                info!("create_drop_in_file_runtime {}", b);
            } else {
                warn!("No action {}", "create_drop_in_file_runtime");
            }
        }

        /* let dialog = flatpak::new("/home/pier/school.txt");
        let window = self.app_window.get().expect("AppWindow supposed to be set");

        dialog.present(Some(window)); */
    }

    pub(super) fn refresh_panels(&self) {
        if self.visible_on_page.get() {
            self.set_file_content_init()
        }
    }

    fn create_drop_in_file(&self, runtime: bool) {
        info!("create_drop_in_file called runtime {runtime}");

        //get the file content
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        let file_path = unit.file_path();
        let primary = unit.primary();
        let file_content = systemd::get_unit_file_info(file_path.as_deref(), &primary)
            .unwrap_or_else(|e| {
                warn!("get_unit_file_info Error: {e:?}");
                "".to_owned()
            });

        let drop_in_file_path = Self::create_drop_in_file_path(&primary, runtime);
        let uri = generate_file_uri(&drop_in_file_path);

        self.file_link.set_uri(&uri);
        self.file_link.set_label(&drop_in_file_path);

        let new_file_content = self
            .set_dropin_file_format(file_path, primary, file_content, runtime)
            .inspect_err(|e| warn!("some error {:?}", e))
            .unwrap_or_default();

        self.set_editor_text(&new_file_content);
        //format the file

        //display
    }

    fn create_drop_in_file_path(primary: &str, runtime: bool) -> String {
        let prefix = if runtime { "run" } else { "etc" };
        format!("/{}/systemd/system/{}.d/override.conf", prefix, primary)
    }

    fn set_dropin_file_format(
        &self,
        file_path: Option<String>,
        primary: String,
        file_content: String,
        runtime: bool,
    ) -> Result<String, SystemdErrors> {
        let mut new_file_content = String::with_capacity(file_content.len() * 2);

        writeln!(
            new_file_content,
            "### Editing {}",
            Self::create_drop_in_file_path(&primary, runtime)
        )?;

        writeln!(
            new_file_content,
            "### Anything between here and the comment below will become the contents of the drop-in file"
        )?;
        new_file_content.push_str("\n\n\n");
        writeln!(
            new_file_content,
            "### Edits below this comment will be discarded"
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
            InterPanelMessage::FileLineNumber(line_number) => self.set_line_number(line_number),
            InterPanelMessage::NewStyleScheme(style_scheme) => {
                self.set_new_style_scheme(style_scheme)
            }
            InterPanelMessage::UnitChange(unit) => self.set_unit(unit),
            _ => {}
        }
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

                let allow_save_condition = unit_file_panel.imp().file_displayed.borrow().is_some(); //TODO check is the text has really changed
                save_button.set_sensitive(allow_save_condition);
            });
        }

        self.sourceview5_buffer
            .set(buffer)
            .expect("sourceview5_buffer set once");
        self.unit_file_text
            .set(view)
            .expect("unit_file_text set once");

        let unit_file_line_number = PREFERENCES.unit_file_line_number();
        self.set_line_number(unit_file_line_number);

        self.file_dropin_selector.connect_n_toggles_notify(|tg| {
            let selected = tg.active();
            info!("selected file {selected}");
        });

        let unit_file_panel = self.obj().clone();
        self.file_dropin_selector.connect_active_notify(move |tg| {
            let selected = tg.active();
            if selected == GTK_INVALID_LIST_POSITION {
                return;
            }

            info!("unit file or drop in: {selected}");
            unit_file_panel
                .imp()
                .file_dropin_selector_activate(selected)
        });
    }
}
impl WidgetImpl for UnitFilePanelImp {}
impl BoxImpl for UnitFilePanelImp {}
