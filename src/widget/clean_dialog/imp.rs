use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gtk::{
    glib::{self, property::PropertySet},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use strum::IntoEnumIterator;

use log::{info, warn};

use crate::{
    systemd::{self, data::UnitInfo, enums::CleanOption, errors::SystemdErrors},
    widget::{app_window::AppWindow, InterPanelAction},
};

use super::CleanDialog;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/clean_dialog.ui")]
pub struct CleanDialogImp {
    #[template_child]
    check_button_box: TemplateChild<gtk::Box>,

    #[template_child]
    clean_button: TemplateChild<gtk::Button>,

    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,

    unit: RefCell<Option<UnitInfo>>,

    check_buttons: OnceCell<HashMap<String, gtk::CheckButton>>,

    app_window: OnceCell<AppWindow>,
}

#[gtk::template_callbacks]
impl CleanDialogImp {
    #[template_callback]
    fn clean_button_clicked(&self, _button: gtk::Button) {
        let unit_binding = self.unit.borrow();
        let Some(unit) = unit_binding.as_ref() else {
            warn!("No unit selected");
            return;
        };

        let Some(map) = self.check_buttons.get() else {
            return;
        };

        let what: Vec<&str> = map
            .iter()
            .filter(|(_, check_button)| check_button.is_active())
            .map(|(clean_option_code, _)| clean_option_code.as_str())
            .collect();

        let plur = if what.len() == 1 { "" } else { "s" };

        let message = match systemd::clean_unit(unit, &what) {
            Ok(()) => {
                format!(
                    "Clean unit <unit>{}</unit> with parameter{} {} succeed",
                    unit.primary(),
                    plur,
                    Self::what_to_display(&what)
                )
            }
            Err(err) => {
                warn!("Clean Unit {:?} error : {:?}", unit.primary(), err);

                self.work_around_dialog(&what, unit, err);
                format!(
                    "Clean unit <unit>{}</unit> with parameter{} {} failed",
                    unit.primary(),
                    plur,
                    Self::what_to_display(&what)
                )
            }
        };

        if let Some(app_window) = self.app_window.get() {
            app_window.add_toast_message(&message, true);
        }
    }

    fn what_to_display(what: &[&str]) -> String {
        let mut out = String::new();

        for (i, w) in what.iter().enumerate() {
            out.push_str("<unit>");
            out.push_str(w);
            out.push_str("</unit>");

            if i + 2 == what.len() {
                out.push_str(" and ");
            } else if i + 1 == what.len() {
                //the last, do nothing
            } else {
                out.push_str(", ");
            }
        }
        out
    }

    pub(crate) fn set_app_window(&self, app_window: Option<&AppWindow>) {
        if let Some(app_window) = app_window {
            self.app_window
                .set(app_window.clone())
                .expect("app_window set once");
        }
    }

    pub(super) fn set_inter_action(&self, _action: &InterPanelAction) {}

    pub fn set_unit(&self, unit: Option<&UnitInfo>) {
        let unit = match unit {
            Some(u) => u,
            None => {
                warn!("set unit to None");
                self.unit.set(None);
                self.window_title.set_subtitle("No Unit Selected");
                return;
            }
        };

        self.unit.set(Some(unit.clone()));

        let label_text = &unit.primary();

        self.window_title.set_subtitle(label_text);

        self.set_send_button_sensitivity();
    }

    pub(super) fn clean_option_selected(&self, _clean_option: &CleanOption, _is_active: bool) {
        self.set_send_button_sensitivity();
    }

    fn set_send_button_sensitivity(&self) {
        if self.unit.borrow().is_none() {
            self.clean_button.set_sensitive(false);
            return;
        }

        let Some(map) = self.check_buttons.get() else {
            return;
        };

        let code_all = CleanOption::All.code();
        if let Some(all) = map.get(code_all) {
            if all.is_active() {
                for (key, check_button) in map.iter() {
                    if key != code_all {
                        check_button.set_active(false);
                    }
                }
            }
        }

        let mut at_least_one_checked = false;
        for check_button in map.values() {
            at_least_one_checked |= check_button.is_active();
        }

        self.clean_button.set_sensitive(at_least_one_checked);
    }

    fn work_around_dialog(&self, what: &[&str], unit: &UnitInfo, err: SystemdErrors) {
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(15)
            .margin_start(5)
            .margin_end(5)
            .margin_top(5)
            .margin_bottom(15)
            .build();

        content_box.append(
            &gtk::Label::builder()
                .label("Unfortunately SysD-Manager can't perfom unit clean currently.")
                .build(),
        );
        content_box.append(
            &gtk::Label::builder()
                .label("Because the app is in construction. Contact me if you know how to manage polkit ;)")
                .build(),
        );
        content_box.append(
            &gtk::Label::builder()
                .label(
                    "\n\nWhile waiting for an eventual fix, please try the bellow command line in your terminal",
                )
                .build(),
        );

        let mut cmd = "sudo systemctl clean ".to_owned();

        for w in what {
            cmd.push_str("--what=");
            cmd.push_str(w);
            cmd.push(' ');
        }

        cmd.push_str(&unit.primary());

        let label_fallback = gtk::Label::builder()
            .label(&cmd)
            .selectable(true)
            .wrap(true)
            .css_classes(["journal_message"])
            .build();

        content_box.append(&label_fallback);

        let tool_bar = adw::ToolbarView::builder().content(&content_box).build();
        tool_bar.add_top_bar(&adw::HeaderBar::new());

        let dialog = adw::Window::builder()
            .title(format!("Error {}", err.human_error_type()))
            .content(&tool_bar)
            .transient_for(self.obj().as_ref())
            .build();

        dialog.present();
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for CleanDialogImp {
    const NAME: &'static str = "CLEAN_DIALOG";
    type Type = CleanDialog;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for CleanDialogImp {
    fn constructed(&self) {
        self.parent_constructed();

        let mut check_buttons = HashMap::new();

        for clean_option in CleanOption::iter() {
            let check_button = gtk::CheckButton::builder()
                .label(clean_option.label())
                .use_underline(true)
                .build();

            let clean_dialog = self.obj().clone();
            check_button.connect_active_notify(move |check_button| {
                info!(
                    "{} is active {}",
                    clean_option.code(),
                    check_button.is_active()
                );

                clean_dialog.clean_option_selected(&clean_option, check_button.is_active());
            });

            self.check_button_box.append(&check_button);

            check_buttons.insert(clean_option.code().to_owned(), check_button);
        }

        self.check_buttons
            .set(check_buttons)
            .expect("check_buttons set once");
    }
}

impl WidgetImpl for CleanDialogImp {}
impl WindowImpl for CleanDialogImp {}
impl AdwWindowImpl for CleanDialogImp {}
