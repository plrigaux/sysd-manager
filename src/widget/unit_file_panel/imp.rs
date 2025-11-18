use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::AdwDialogExt;
use gtk::{
    TemplateChild,
    ffi::GTK_INVALID_LIST_POSITION,
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

use log::{debug, info, warn};
use sourceview5::{Buffer, prelude::*};

use crate::{
    consts::{ADWAITA, SUGGESTED_ACTION},
    systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
    utils::font_management::set_text_view_font_display,
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::{data::PREFERENCES, style_scheme::style_schemes},
    },
};

use super::flatpak;

const PANEL_EMPTY: &str = "empty";
const PANEL_FILE: &str = "file_panel";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
pub struct UnitFilePanelImp {
    #[template_child]
    save_button: TemplateChild<gtk::Button>,

    /*    #[template_child]
    unit_file_text: TemplateChild<gtk::TextView>, */
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

    app_window: OnceCell<AppWindow>,

    visible_on_page: Cell<bool>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,

    drop_in_files: RefCell<Vec<String>>,

    file_content_selected_index: Cell<u32>,
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

        match systemd::save_text_to_file(unit, &text) {
            Ok((file_path, _bytes_written)) => {
                button.remove_css_class(SUGGESTED_ACTION);
                let msg = format!("File <u>{file_path}</u> saved successfully!");
                self.add_toast_message(&msg, true);
            }
            Err(error) => {
                warn!(
                    "Unable to save file: {:?}, Error {:?}",
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
        let file_content = systemd::get_unit_file_info(file_path.as_deref(), primary)
            .unwrap_or_else(|e| {
                warn!("get_unit_file_info Error: {e:?}");
                "".to_owned()
            });

        let file_path = file_path.unwrap_or_default();

        let uri = generate_file_uri(&file_path);

        self.file_link.set_uri(&uri);

        self.file_link.set_label(&file_path);

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
            return;
        }

        let toggle = adw::Toggle::builder()
            .label("Unit File")
            .name("file")
            .build();
        self.file_dropin_selector.add(toggle);

        for (idx, drop_in_file) in drop_in_files.iter().enumerate() {
            let label = if drop_in_files.len() > 1 {
                format!("Drop In {idx}")
            } else {
                "Drop In".to_owned()
            };
            let toggle = adw::Toggle::builder()
                .label(label)
                .name(format!("dropin {idx}"))
                .tooltip(drop_in_file)
                .build();
            self.file_dropin_selector.add(toggle);
        }

        self.set_visible();
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

        self.save_button.remove_css_class(SUGGESTED_ACTION);
        self.set_visible();
    }

    fn set_visible(&self) {
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
        self.app_window
            .set(app_window.clone())
            .expect("toast_overlay once");

        /* let dialog = flatpak::new("/home/pier/school.txt");
        let window = self.app_window.get().expect("AppWindow supposed to be set");

        dialog.present(Some(window)); */
    }

    pub(super) fn refresh_panels(&self) {
        if self.visible_on_page.get() {
            self.set_file_content_init()
        }
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
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for UnitFilePanelImp {
    fn constructed(&self) {
        self.parent_constructed();

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
        {
            let buffer = view.buffer();

            let save_button = self.save_button.clone();
            buffer.connect_begin_user_action(move |_buf| {
                save_button.add_css_class(SUGGESTED_ACTION);
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
