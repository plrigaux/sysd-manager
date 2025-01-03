pub mod dosini;
pub mod flatpak;

use gtk::{glib, subclass::prelude::ObjectSubclassIsExt};
use log::warn;

use crate::systemd::{self, data::UnitInfo};

use super::app_window::AppWindow;

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitFilePanel(ObjectSubclass<imp::UnitFilePanelImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitFilePanel {
    pub fn new() -> Self {
        // Create new window
        let obj: UnitFilePanel = glib::Object::new();

        let system_manager = adw::StyleManager::default();

        let is_dark = system_manager.is_dark();

        obj.set_dark(is_dark);

        obj
    }

    pub fn set_file_content(&self, unit: &UnitInfo) {
        let file_content = match systemd::get_unit_file_info(&unit) {
            Ok(content) => content,
            Err(e) => {
                warn!("get_unit_file_info Error: {:?}", e);
                "".to_owned()
            }
        };

        let file_path = unit.file_path().map_or("".to_owned(), |a| a);
        self.imp()
            .display_file_info(&file_path, &file_content, unit);
    }

    pub fn set_dark(&self, is_dark: bool) {
        self.imp().set_dark(is_dark)
    }

    pub fn register(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        self.imp().register(app_window, toast_overlay);
    }
}

mod imp {
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

    use log::{info, warn};

    use crate::{
        systemd::{self, data::UnitInfo, errors::SystemdErrors, generate_file_uri},
        widget::{app_window::AppWindow, preferences::data::PREFERENCES},
    };

    use super::{dosini, flatpak};

    const SUGGESTED_ACTION: &str = "suggested-action";

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_file_panel.ui")]
    pub struct UnitFilePanelImp {
        #[template_child]
        save_button: TemplateChild<gtk::Button>,

        #[template_child]
        unit_file_text: TemplateChild<gtk::TextView>,

        #[template_child]
        file_link: TemplateChild<gtk::LinkButton>,

        toast_overlay: OnceCell<adw::ToastOverlay>,

        app_window: OnceCell<AppWindow>,

        unit: RefCell<Option<UnitInfo>>,

        is_dark: Cell<bool>,
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
                        SystemdErrors::CmdNoFreedesktopFlatpakPermission(
                            command_line,
                            file_path,
                        ) => {
                            let dialog = flatpak::new(command_line, file_path);
                            let window =
                                self.app_window.get().expect("AppWindow supposed to be set");

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

        pub(crate) fn display_file_info(
            &self,
            file_path: &str,
            file_content: &str,
            unit: &UnitInfo,
        ) {
            let uri = generate_file_uri(file_path);

            self.file_link.set_uri(&uri);

            self.file_link.set_label(file_path);

            let _old = self.unit.replace(Some(unit.clone()));

            self.set_text(file_content);
        }

        fn set_text(&self, file_content: &str) {
            let in_color = PREFERENCES.unit_file_colors();

            let buf = self.unit_file_text.buffer();
            if in_color {
                buf.set_text("");

                let is_dark = self.is_dark.get();
                let mut start_iter = buf.start_iter();

                let text = dosini::convert_to_mackup(&file_content, is_dark);
                buf.insert_markup(&mut start_iter, &text);
            } else {
                buf.set_text(&file_content);
            }

            self.save_button.remove_css_class(SUGGESTED_ACTION);
        }

        pub(crate) fn set_dark(&self, is_dark: bool) {
            self.is_dark.set(is_dark);

            //get current text
            let buffer = self.unit_file_text.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let file_content = buffer.text(&start, &end, true);

            self.set_text(file_content.as_str());
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
}
