use std::cell::{Cell, OnceCell, RefCell};

use adw::prelude::AdwDialogExt;
use gtk::{
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
    TemplateChild,
};

use log::{debug, info, warn};
use sourceview5::{prelude::*, Buffer};

use crate::{
    consts::SUGGESTED_ACTION,
    systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
    utils::font_management::set_text_view_font_display,
    widget::{
        app_window::AppWindow,
        preferences::{data::PREFERENCES, style_scheme::style_schemes},
        InterPanelAction,
    },
};

use super::flatpak;

const ADWAITA: &str = "Adwaita";
const PANEL_EMPTY: &str = "empty";
const PANEL_FILE: &str = "file_panel";

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
#[properties(wrapper_type = super::UnitFilePanel)]
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
    panel_file_stack: TemplateChild<gtk::Stack>,
    toast_overlay: OnceCell<adw::ToastOverlay>,

    app_window: OnceCell<AppWindow>,

    visible_on_page: Cell<bool>,

    #[property(get, set=Self::set_unit, nullable)]
    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,
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
        info!("button {:?}", button);

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
                let msg = format!("File <u>{file_path}</u> saved succesfully!");
                let toast = adw::Toast::builder().use_markup(true).title(&msg).build();
                self.toast_overlay.get().unwrap().add_toast(toast)
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
                        let toast = adw::Toast::builder()
                            .use_markup(true)
                            .title("Not able to save file, permission not granted!")
                            .build();
                        self.toast_overlay.get().unwrap().add_toast(toast);
                    }
                    _ => {
                        let toast = adw::Toast::builder()
                            .use_markup(true)
                            .title("Not able to save file, an error happened!")
                            .build();
                        self.toast_overlay.get().unwrap().add_toast(toast)
                    }
                }
            }
        };
    }
}

impl UnitFilePanelImp {
    fn set_visible_on_page(&self, value: bool) {
        debug!("set_visible_on_page val {value}");
        self.visible_on_page.set(value);

        if self.visible_on_page.get()
            && !self.unit_dependencies_loaded.get()
            && self.unit.borrow().is_some()
        {
            self.set_file_content()
        }
    }

    fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                self.unit.replace(None);
                self.set_file_content();
                return;
            }
        };

        let old_unit = self.unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit {
            if old_unit.primary() != unit.primary() {
                self.unit_dependencies_loaded.set(false)
            }
        }

        self.set_file_content()
    }

    pub fn set_file_content(&self) {
        if !self.visible_on_page.get() {
            return;
        }

        let binding = self.unit.borrow();
        let Some(unit_ref) = binding.as_ref() else {
            warn!("No unit file");
            self.set_text("");

            return;
        };

        let file_content = match systemd::get_unit_file_info(unit_ref) {
            Ok(content) => content,
            Err(e) => {
                warn!("get_unit_file_info Error: {:?}", e);
                "".to_owned()
            }
        };

        let file_path = unit_ref.file_path().map_or("".to_owned(), |a| a);

        let uri = generate_file_uri(&file_path);

        self.file_link.set_uri(&uri);

        self.file_link.set_label(&file_path);

        self.set_text(&file_content);
    }

    fn set_text(&self, file_content: &str) {
        let buf = self
            .unit_file_text
            .get()
            .expect("expect sourceview5::View")
            .buffer();

        buf.set_text(""); //To clear current
        buf.set_text(file_content);

        self.save_button.remove_css_class(SUGGESTED_ACTION);

        let panel = if file_content.is_empty() {
            PANEL_EMPTY
        } else {
            PANEL_FILE
        };

        //  self.panel_file_stack.set_visible_child_name(panel);
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);

        let style_scheme_id = PREFERENCES.unit_file_style_scheme();

        debug!(
            "File Unit set_dark {is_dark} style_scheme_id {:?}",
            style_scheme_id
        );

        self.set_new_style_scheme(Some(&style_scheme_id));
    }

    fn set_highlight(&self, highlight: bool) {
        if highlight {
            let style_scheme_id = PREFERENCES.unit_file_style_scheme();
            self.set_new_style_scheme(Some(&style_scheme_id));
        } else {
            self.set_new_style_scheme(None);
        }
    }

    fn set_new_style_scheme(&self, mut style_scheme_id: Option<&str>) {
        if !PREFERENCES.unit_file_highlight() {
            style_scheme_id = None
        }

        info!("Set new style scheme {:?}", style_scheme_id);
        if let Some(mut style_scheme_id) = style_scheme_id {
            let style_schemes_map = style_schemes();

            debug!("{:#?}", style_schemes_map);
            if style_scheme_id.is_empty() {
                style_scheme_id = ADWAITA;
            }

            let style_scheme_st = style_schemes_map.get(style_scheme_id);

            let style_sheme_st = match style_scheme_st {
                Some(ss) => ss,
                None => {
                    warn!(
                        "style scheme id \"{style_scheme_id}\" not found in {:?}",
                        style_schemes_map.keys().collect::<Vec<_>>()
                    );

                    //fallback on style Adwaita
                    if let Some(style_scheme_st) = style_schemes_map.get(ADWAITA) {
                        style_scheme_st
                    } else
                    //fallback on first item
                    if let Some((_, style_scheme_st)) = style_schemes_map.first_key_value() {
                        style_scheme_st
                    } else {
                        return;
                    }
                }
            };

            let scheme_id = &style_sheme_st.get_style_scheme_id(self.is_dark.get());

            if let Some(ref scheme) = sourceview5::StyleSchemeManager::new().scheme(scheme_id) {
                let buffer = get_buffer!(self);
                info!("Style Scheme found for id {:?}", scheme_id);
                buffer.set_style_scheme(Some(scheme));
            } else {
                warn!("No Style Scheme found for id {:?}", scheme_id)
            }
        } else {
            let buffer = get_buffer!(self);

            buffer.set_style_scheme(None);
        }
    }

    pub(crate) fn register(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.toast_overlay
            .set(toast_overlay.clone())
            .expect("toast_overlay once");

        self.app_window
            .set(app_window.clone())
            .expect("toast_overlay once");

        /* let dialog = flatpak::new("/home/pier/school.txt");
        let window = self.app_window.get().expect("AppWindow supposed to be set");

        dialog.present(Some(window)); */
    }

    pub(super) fn refresh_panels(&self) {
        if self.visible_on_page.get() {
            self.set_file_content()
        }
    }

    pub(super) fn set_inter_action(&self, action: &InterPanelAction) {
        match *action {
            InterPanelAction::FontProvider(old, new) => {
                let view = self.unit_file_text.get().expect("expect sourceview5::View");

                set_text_view_font_display(old, new, &view.display())
            }
            InterPanelAction::IsDark(is_dark) => self.set_dark(is_dark),
            InterPanelAction::PanelVisible(visible) => self.set_visible_on_page(visible),
            InterPanelAction::FileHighlighting(highlight) => self.set_highlight(highlight),
            InterPanelAction::NewStyleScheme(style_scheme) => {
                self.set_new_style_scheme(style_scheme)
            }
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

#[glib::derived_properties]
impl ObjectImpl for UnitFilePanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        let buffer = sourceview5::Buffer::new(None);

        if PREFERENCES.unit_file_highlight() {
            if let Some(ref language) = sourceview5::LanguageManager::new().language("ini") {
                buffer.set_language(Some(language));
            }
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

        let _ = self.sourceview5_buffer.set(buffer);
        let _ = self.unit_file_text.set(view);
    }
}
impl WidgetImpl for UnitFilePanelImp {}
impl BoxImpl for UnitFilePanelImp {}
