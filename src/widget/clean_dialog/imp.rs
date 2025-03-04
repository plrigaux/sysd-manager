use std::{
    cell::{Cell, OnceCell, RefCell},
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
    systemd::{data::UnitInfo, enums::CleanOption},
    widget::InterPanelAction,
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

    is_dark: Cell<bool>,

    check_buttons: OnceCell<HashMap<String, gtk::CheckButton>>,
}

#[gtk::template_callbacks]
impl CleanDialogImp {
    #[template_callback]
    fn clean_button_clicked(&self, _button: gtk::Button) {}

    pub(super) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    pub(super) fn set_inter_action(&self, action: &InterPanelAction) {
        if let InterPanelAction::IsDark(is_dark) = *action {
            self.set_dark(is_dark)
        }
    }

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

        let _ = self.check_buttons.set(check_buttons);
    }
}

impl WidgetImpl for CleanDialogImp {}
impl WindowImpl for CleanDialogImp {}
impl AdwWindowImpl for CleanDialogImp {}
