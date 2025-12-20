use std::{cell::OnceCell, collections::HashMap};

use adw::{prelude::*, subclass::window::AdwWindowImpl};
use gettextrs::pgettext;
use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use strum::IntoEnumIterator;

use crate::{
    systemd::{self, data::UnitInfo, enums::CleanOption, errors::SystemdErrors},
    widget::unit_control_panel::{UnitControlPanel, work_around_dialog},
};
use base::enums::UnitDBusLevel;
use log::{info, warn};

use super::CleanUnitDialog;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/clean_dialog.ui")]
pub struct CleanDialogImp {
    #[template_child]
    check_button_box: TemplateChild<gtk::Box>,

    #[template_child]
    clean_button: TemplateChild<gtk::Button>,

    #[template_child]
    window_title: TemplateChild<adw::WindowTitle>,

    check_buttons: OnceCell<HashMap<String, gtk::CheckButton>>,

    unit_control: OnceCell<UnitControlPanel>,
}

#[gtk::template_callbacks]
impl CleanDialogImp {
    #[template_callback]
    fn clean_button_clicked(&self, button: gtk::Button) {
        let Some(map) = self.check_buttons.get() else {
            return;
        };

        let what: Vec<String> = map
            .iter()
            .filter(|(_, check_button)| check_button.is_active())
            .map(|(clean_option_code, _)| clean_option_code.clone())
            .collect();

        let lambda_out = {
            let what = what.clone();
            let this = self.obj().clone();
            move |method: &str,
                  unit: Option<&UnitInfo>,
                  result: Result<(), SystemdErrors>,
                  _control: &UnitControlPanel| {
                if let Err(error) = result
                    && let SystemdErrors::ZAccessDenied(_, _) = error
                {
                    let mut cmd = "sudo systemctl clean ".to_owned();

                    for w in what {
                        cmd.push_str("--what=");
                        cmd.push_str(&w);
                        cmd.push(' ');
                    }

                    cmd.push_str(&unit.expect("Unit not None").primary());
                    work_around_dialog(&cmd, &error, method, &this.into())
                }
            }
        };

        let lambda = move |params: Option<(UnitDBusLevel, String)>| {
            if let Some((level, primary_name)) = params {
                systemd::clean_unit(level, &primary_name, &what)
            } else {
                Err(SystemdErrors::NoUnit)
            }
        };

        self.unit_control
            .get()
            .expect("unit_control not None")
            .call_method(
                /*Message answer*/ &pgettext("clean", "Clean"),
                true,
                &button,
                lambda,
                lambda_out,
            );
    }

    pub(crate) fn set_unit_control_panel(&self, unit_control: &UnitControlPanel) {
        let _ = self.unit_control.set(unit_control.clone());

        let sub_title = match unit_control.current_unit() {
            Some(u) => u.primary(),
            None => {
                warn!("set unit to None");
                pgettext("clean", "No Unit Selected")
            }
        };

        self.window_title.set_subtitle(&sub_title);

        self.set_send_button_sensitivity();
    }

    pub(super) fn clean_option_selected(&self, _clean_option: &CleanOption, _is_active: bool) {
        self.set_send_button_sensitivity();
    }

    fn set_send_button_sensitivity(&self) {
        if self
            .unit_control
            .get()
            .and_then(|unit_control| unit_control.current_unit())
            .is_none()
        {
            self.clean_button.set_sensitive(false);
            return;
        }

        let Some(map) = self.check_buttons.get() else {
            return;
        };

        let code_all = CleanOption::All.code();
        if let Some(all) = map.get(code_all)
            && all.is_active()
        {
            for (key, check_button) in map.iter() {
                if key != code_all {
                    check_button.set_active(false);
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
    type Type = CleanUnitDialog;
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

                clean_dialog
                    .imp()
                    .clean_option_selected(&clean_option, check_button.is_active());
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
