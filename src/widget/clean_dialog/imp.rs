use std::cell::{Cell, OnceCell, RefCell};

use adw::{
    prelude::*,
    subclass::{window::AdwWindowImpl, *},
};
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

use log::warn;

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

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,
}

#[gtk::template_callbacks]
impl CleanDialogImp {
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

                return;
            }
        };

        self.unit.set(Some(unit.clone()));
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

        for clean in CleanOption::iter() {
            let check_button = gtk::CheckButton::builder()
                .label(clean.label())
                .use_underline(true)
                .build();

            self.check_button_box.append(&check_button);
        }
    }
}

impl WidgetImpl for CleanDialogImp {}
impl WindowImpl for CleanDialogImp {}
impl AdwWindowImpl for CleanDialogImp {}
