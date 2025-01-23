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

use crate::{
    consts::SUGGESTED_ACTION,
    systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
    widget::{app_window::AppWindow, preferences::data::PREFERENCES},
};

use super::{dosini, flatpak};

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
#[properties(wrapper_type = super::UnitFilePanel)]
pub struct UnitFilePanelImp {
    #[template_child]
    save_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_file_text: TemplateChild<gtk::TextView>,

    #[template_child]
    file_link: TemplateChild<gtk::LinkButton>,

    toast_overlay: OnceCell<adw::ToastOverlay>,

    app_window: OnceCell<AppWindow>,

    #[property(get, set=Self::set_visible_on_page)]
    visible_on_page: Cell<bool>,

    #[property(get, set=Self::set_unit, nullable)]
    unit: RefCell<Option<UnitInfo>>,

    #[property(get, set)]
    dark: Cell<bool>,

    unit_dependencies_loaded: Cell<bool>,
}

#[gtk::template_callbacks]
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

    #[template_callback]
    fn save_file(&self, button: &gtk::Button) {
        info!("button {:?}", button);

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        let buffer = self.unit_file_text.buffer();
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

    fn set_text(&self, file_content: &str) {
        let in_color = PREFERENCES.unit_file_colors();

        let buf = self.unit_file_text.buffer();
        if in_color {
            buf.set_text("");

            let is_dark = self.dark.get();
            let mut start_iter = buf.start_iter();

            let text = dosini::convert_to_mackup(file_content, is_dark);
            buf.insert_markup(&mut start_iter, &text);
        } else {
            buf.set_text(file_content);
        }

        self.save_button.remove_css_class(SUGGESTED_ACTION);
    }
    /*
    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);

        //get current text
        let buffer = self.unit_file_text.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let file_content = buffer.text(&start, &end, true);

        self.set_text(file_content.as_str());
    } */

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
        {
            let buffer = self.unit_file_text.buffer();
            let save_button = self.save_button.clone();
            buffer.connect_begin_user_action(move |_buf| {
                save_button.add_css_class(SUGGESTED_ACTION);
            });
        }
    }
}
impl WidgetImpl for UnitFilePanelImp {}
impl BoxImpl for UnitFilePanelImp {}
