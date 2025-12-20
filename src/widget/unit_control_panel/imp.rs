use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

use adw::{prelude::*, subclass::prelude::*};
use gettextrs::pgettext;
use gtk::{
    gio,
    glib::{self},
    pango::{self, FontDescription},
};
use log::{debug, info, warn};

use super::{
    UnitControlPanel, controls, enums::UnitContolType, side_control_panel::SideControlPanel,
};
use crate::{
    consts::{DESTRUCTIVE_ACTION, SUGGESTED_ACTION},
    format2,
    systemd::{
        self,
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus, StartStopMode},
        errors::SystemdErrors,
    },
    utils::{
        font_management::{self, FONT_CONTEXT, create_provider},
        palette::{blue, red},
    },
    widget::{
        InterPanelMessage, app_window::AppWindow, journal::JournalPanel,
        preferences::data::PREFERENCES, unit_dependencies_panel::UnitDependenciesPanel,
        unit_file_panel::UnitFilePanel, unit_info::UnitInfoPanel,
    },
};
use base::enums::UnitDBusLevel;
use strum::IntoEnumIterator;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_control_panel.ui")]
#[properties(wrapper_type = super::UnitControlPanel)]
pub struct UnitControlPanelImpl {
    #[template_child]
    unit_info_panel: TemplateChild<UnitInfoPanel>,

    #[template_child]
    unit_dependencies_panel: TemplateChild<UnitDependenciesPanel>,

    #[template_child]
    unit_file_panel: TemplateChild<UnitFilePanel>,

    #[template_child]
    unit_journal_panel: TemplateChild<JournalPanel>,

    #[template_child]
    ablement_switch: TemplateChild<gtk::Switch>,

    #[template_child]
    start_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    stop_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    show_more_button: TemplateChild<gtk::MenuButton>,

    #[template_child]
    restart_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    more_action_popover: TemplateChild<gtk::Popover>,

    #[template_child]
    start_modes: TemplateChild<gtk::Box>,

    #[template_child]
    stop_modes: TemplateChild<gtk::Box>,

    #[template_child]
    restart_modes: TemplateChild<gtk::Box>,

    #[template_child]
    unit_panel_stack: TemplateChild<adw::ViewStack>,

    app_window: OnceCell<AppWindow>,
    more_action_panel: OnceCell<SideControlPanel>,
    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,

    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,

    old_font_provider: RefCell<Option<gtk::CssProvider>>,

    is_dark: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for UnitControlPanelImpl {
    const NAME: &'static str = "UnitControlPanel";
    type Type = super::UnitControlPanel;
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

macro_rules! current_unit {
    ($app:expr) => {{ current_unit!($app, ()) }};

    ($app:expr, $opt:expr) => {{
        let unit_op = $app.current_unit.borrow();
        let Some(unit) = unit_op.as_ref() else {
            warn!("No selected unit!");
            return $opt;
        };

        unit.clone()
    }};
}

#[gtk::template_callbacks]
impl UnitControlPanelImpl {
    pub(super) fn set_overlay(&self, app_window: &AppWindow) {
        //self.kill_panel.register(&self.side_overlay, toast_overlay);
        self.unit_file_panel.register(app_window);
        self.unit_dependencies_panel.register(app_window);
        self.unit_info_panel.register(app_window);

        if let Some(side_panel) = self.more_action_panel.get() {
            side_panel.set_app_window(app_window);
        } else {
            warn!("Side Panel Should not be None");
        }

        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");
    }

    pub fn app_window(&self) -> Option<AppWindow> {
        self.app_window.get().cloned()
    }

    #[template_callback]
    fn switch_ablement_state_set(&self, switch_new_state: bool, switch: &gtk::Switch) -> bool {
        info!(
            "switch_ablement_state_set new {switch_new_state} old {}",
            switch.state()
        );

        if switch_new_state == switch.state() {
            debug!("no state change");
            return true;
        }

        let unit = current_unit!(self, true);

        let expected_new_status = if switch_new_state {
            EnablementStatus::Enabled
        } else {
            EnablementStatus::Disabled
        };

        controls::switch_ablement_state_set(
            &self.obj(),
            expected_new_status,
            switch,
            &unit,
            self.is_dark.get(),
            Rc::new(Box::new(|| {})),
        );

        self.unit_info_panel
            .set_inter_message(&InterPanelMessage::UnitChange(Some(&unit)));
        true // to stop the signal emission
    }

    #[template_callback]
    fn button_start_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::start_unit,
            UnitContolType::Start,
            None,
            Rc::new(Box::new(move || {})),
        );
    }

    #[template_callback]
    fn button_stop_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::stop_unit,
            UnitContolType::Stop,
            None,
            Rc::new(Box::new(move || {})),
        );
    }

    #[template_callback]
    fn button_restart_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::restart_unit,
            UnitContolType::Restart,
            None,
            Rc::new(Box::new(move || {})),
        );
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.search_bar
            .borrow()
            .set_search_mode(toggle_button.is_active());
    }
}

impl UnitControlPanelImpl {
    //Dry
    fn start_restart_action(
        &self,
        button: &impl IsA<gtk::Widget>,
        systemd_method: fn(UnitDBusLevel, &str, StartStopMode) -> Result<String, SystemdErrors>,
        action: UnitContolType,

        unit: Option<&UnitInfo>,
        call_back: Rc<Box<dyn Fn()>>,
    ) {
        let unit = if let Some(unit) = unit {
            unit.clone()
        } else {
            current_unit!(self)
        };

        let start_mode: StartStopMode = (&self.start_mode).into();

        let unit_control_panel = self.obj().clone();

        let button = button.clone();
        let primary_name = unit.primary();
        let level = unit.dbus_level();
        glib::spawn_future_local(async move {
            button.set_sensitive(false);

            let start_results =
                gio::spawn_blocking(move || systemd_method(level, &primary_name, start_mode))
                    .await
                    .expect("Task needs to finish successfully.");

            button.set_sensitive(true);

            unit_control_panel.imp().start_restart(
                &unit.primary(),
                Some(&unit),
                start_results,
                action,
                start_mode,
            );

            call_back();
        });
    }

    pub(super) fn start_restart(
        &self,
        unit_name: &str,
        unit_op: Option<&UnitInfo>,
        start_results: Result<String, SystemdErrors>,
        action: UnitContolType,
        mode: StartStopMode,
    ) {
        let job_op = match start_results {
            Ok(job) => {
                info!(
                    "{} SUCCESS, Unit {:?} {:?}",
                    action.code(),
                    unit_name,
                    mode.as_str()
                );

                let red_green = if action != UnitContolType::Stop {
                    "green"
                } else {
                    "red"
                };

                let info = format2!(
                    //toast message
                    pgettext(
                        "toast",
                        "Unit <unit>{}</unit> has been <{0}>{}</{0}> with the mode <unit>{}</unit>"
                    ),
                    red_green,
                    unit_name,
                    action.past_participle(),
                    mode.as_str()
                );

                self.add_toast_message(&info, true);

                if let Some(unit) = unit_op {
                    debug!("State-A {}", unit.active_state());

                    if let Ok(new_unit) = systemd::fetch_unit(unit.dbus_level(), &unit.primary()) {
                        unit.set_active_state(new_unit.active_state());
                    }

                    debug!("State-B {}", unit.active_state());
                    self.highlight_controls(unit);
                }

                Some(job)
            }
            Err(err) => {
                warn!("{} FAILED, Unit {:?} {:?}", action.code(), unit_name, err);

                let info = format2!(
                    //toast message error --  "Can't {ACTION} the unit <unit>{UNITNAME}</unit>, because: {SYSTEMD HUMAN ERROR (english)}"),
                    pgettext("toast", "Can't {} the unit <unit>{}</unit>, because: {}"),
                    action.label(),
                    unit_name,
                    err.human_error_type()
                );

                self.add_toast_message(&info, true);

                None
            }
        };

        let Some(_job) = job_op else {
            return;
        };

        self.unit_info_panel
            .set_inter_message(&InterPanelMessage::UnitChange(unit_op));
    }

    pub(super) fn selection_change(&self, unit: Option<&UnitInfo>) {
        let action = InterPanelMessage::UnitChange(unit);
        self.set_inter_message(&action);
        self.unit_info_panel
            .set_inter_message(&InterPanelMessage::UnitChange(unit));
        self.unit_file_panel
            .set_inter_message(&InterPanelMessage::UnitChange(unit));
        self.unit_journal_panel
            .set_inter_message(&InterPanelMessage::UnitChange(unit));
        self.unit_dependencies_panel
            .set_inter_message(&InterPanelMessage::UnitChange(unit));

        let unit = match unit {
            Some(u) => u,
            None => {
                self.current_unit.replace(None);
                return;
            }
        };

        let old_unit = self.current_unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit
            && old_unit.primary() == unit.primary()
        {
            info! {"Same unit {}", unit.primary() };
            /*                 self.highlight_controls(unit);
            return; */
        }

        controls::handle_switch_sensivity(&self.ablement_switch, unit, true, self.is_dark.get());

        self.start_button.set_sensitive(true);
        self.stop_button.set_sensitive(true);
        self.restart_button.set_sensitive(true);
        //self.kill_button.set_sensitive(true);

        self.highlight_controls(unit);
    }

    pub(super) fn refresh_panels(&self) {
        let binding = self.current_unit.borrow();
        if let Some(unit) = binding.as_ref() {
            self.highlight_controls(unit);

            self.unit_file_panel.refresh_panels();
            self.unit_info_panel.refresh_panels();
            self.unit_journal_panel.refresh_panels();
        }
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    pub fn set_inter_message(&self, action: &InterPanelMessage) {
        match action {
            InterPanelMessage::Font(font_description) => {
                let provider = create_provider(font_description);
                {
                    let binding = self.old_font_provider.borrow();
                    let old_provider = binding.as_ref();
                    let new_action =
                        InterPanelMessage::FontProvider(old_provider, provider.as_ref());
                    self.forward_inter_actions(&new_action);
                }
                self.old_font_provider.replace(provider);
            }
            InterPanelMessage::IsDark(is_dark) => {
                self.set_dark(*is_dark);
                self.forward_inter_actions(action)
            }
            InterPanelMessage::JournalFilterBoot(_) => {
                self.display_journal_page();
                self.forward_inter_actions(action)
            }

            InterPanelMessage::StartUnit(button, unit, call_back) => {
                self.start_restart_action(
                    button,
                    systemd::start_unit,
                    UnitContolType::Start,
                    Some(unit),
                    call_back.clone(),
                );
            }
            InterPanelMessage::StopUnit(button, unit, call_back) => {
                self.start_restart_action(
                    button,
                    systemd::stop_unit,
                    UnitContolType::Stop,
                    Some(unit),
                    call_back.clone(),
                );
            }
            InterPanelMessage::ReStartUnit(button, unit, call_back) => {
                self.start_restart_action(
                    button,
                    systemd::restart_unit,
                    UnitContolType::Restart,
                    Some(unit),
                    call_back.clone(),
                );
            }
            InterPanelMessage::EnableUnit(unit, call_back) => {
                controls::switch_ablement_state_set(
                    &self.obj(),
                    EnablementStatus::Enabled,
                    &self.ablement_switch,
                    unit,
                    self.is_dark.get(),
                    call_back.clone(),
                );
            }
            InterPanelMessage::DisableUnit(unit, call_back) => {
                controls::switch_ablement_state_set(
                    &self.obj(),
                    EnablementStatus::Disabled,
                    &self.ablement_switch,
                    unit,
                    self.is_dark.get(),
                    call_back.clone(),
                );
            }

            InterPanelMessage::ReenableUnit(unit, call_back) => {
                controls::reeenable_unit(
                    &self.obj(),
                    &self.ablement_switch,
                    unit,
                    self.is_dark.get(),
                    call_back.clone(),
                );
            }

            InterPanelMessage::ReloadUnit(button, unit, call_back) => {
                self.start_restart_action(
                    button,
                    systemd::reload_unit,
                    UnitContolType::Reload,
                    Some(unit),
                    call_back.clone(),
                );
            }
            _ => self.forward_inter_actions(action),
        }
    }

    fn forward_inter_actions(&self, action: &InterPanelMessage) {
        self.unit_info_panel.set_inter_message(action);
        self.unit_dependencies_panel.set_inter_message(action);
        self.unit_file_panel.set_inter_message(action);
        self.unit_journal_panel.set_inter_message(action);

        /*   let Some(side_panel) = self.more_action_panel.get() else {
            warn!("Side Panel Should not be None");
            return;
        };

        side_panel.set_inter_message(action); */
    }

    //TODO bind to the property
    fn highlight_controls(&self, unit: &UnitInfo) {
        match unit.active_state() {
            ActiveState::Active
            | ActiveState::Activating
            | ActiveState::Reloading
            | ActiveState::Refreshing => {
                self.stop_button.add_css_class(DESTRUCTIVE_ACTION);
                self.start_button.remove_css_class(SUGGESTED_ACTION);
            }
            ActiveState::Inactive | ActiveState::Deactivating => {
                self.stop_button.remove_css_class(DESTRUCTIVE_ACTION);
                self.start_button.add_css_class(SUGGESTED_ACTION);
            }
            _ => {
                self.stop_button.remove_css_class(DESTRUCTIVE_ACTION);
                self.start_button.remove_css_class(SUGGESTED_ACTION);
            }
        }
    }

    fn set_modes(&self, modes_box: &gtk::Box, control_type: UnitContolType) {
        let default = StartStopMode::Fail;
        let mut ck_group: Option<gtk::CheckButton> = None;

        for mode in StartStopMode::iter() {
            if control_type == UnitContolType::Stop && mode == StartStopMode::Isolate {
                continue;
            }

            let ck = gtk::CheckButton::builder().label(mode.as_str()).build();

            modes_box.append(&ck);

            let source_property = format!("{}_mode", control_type.code());
            let unit_control_panel = self.obj();
            ck.bind_property(
                "active",
                &unit_control_panel as &UnitControlPanel,
                &source_property,
            )
            .transform_to(move |_, _active: bool| Some(mode.as_str()))
            .build();

            if mode == default {
                ck.set_active(true);
            }

            if ck_group.is_none() {
                ck_group = Some(ck);
            } else {
                ck.set_group(ck_group.as_ref());
            }
        }
    }

    pub(super) fn display_info_page(&self) {
        self.unit_panel_stack.set_visible_child_name("info_page");
    }

    pub(super) fn display_dependencies_page(&self) {
        self.unit_panel_stack
            .set_visible_child_name("dependencies_page");
    }

    pub(super) fn display_journal_page(&self) {
        self.unit_panel_stack.set_visible_child_name("journal_page");
    }

    pub fn display_definition_file_page(&self) {
        self.unit_panel_stack
            .set_visible_child_name("definition_file_page");
    }

    /*     pub fn unlink_child(&self, is_signal: bool) {
        let Some(side_panel) = self.more_action_panel.get() else {
            warn!("Side Panel Should not be None");
            return;
        };
        side_panel.unlink_child(is_signal);
    } */

    pub(super) fn add_toast_message(&self, message: &str, use_markup: bool) {
        if let Some(app_window) = self.app_window.get() {
            app_window.add_toast_message(message, use_markup, None);
        }
    }

    pub fn parent_window(&self) -> gtk::Window {
        let w: gtk::Window = self.app_window.get().expect("window set").clone().into();
        w
    }

    pub(super) fn call_method<T>(
        &self,
        method_name: &str,
        need_selected_unit: bool,
        button: &impl IsA<gtk::Widget>,
        systemd_method: impl Fn(Option<(UnitDBusLevel, String)>) -> Result<T, SystemdErrors>
        + std::marker::Send
        + 'static,
        return_handle: impl FnOnce(&str, Option<&UnitInfo>, Result<T, SystemdErrors>, &UnitControlPanel)
        + 'static,
    ) where
        T: Send + 'static,
    {
        let binding = self.current_unit.borrow();
        let unit_option = binding.clone();

        if need_selected_unit && unit_option.is_none() {
            warn!("No Unit");
            return;
        };

        //TODO investigate
        let is_dark = true; //self.is_dark.get();
        let blue = blue(is_dark).get_color();

        let control_panel: UnitControlPanel = self.obj().clone();
        let button = button.clone();
        let method_name = method_name.to_owned();

        let params = if let Some(ref unit) = unit_option {
            let primary_name = unit.primary();
            let level = unit.dbus_level();

            Some((level, primary_name))
        } else {
            None
        };

        //   let systemd_method = systemd_method.clone();
        glib::spawn_future_local(async move {
            button.set_sensitive(false);

            let result = gio::spawn_blocking(move || systemd_method(params))
                .await
                .expect("Call needs to finish successfully.");

            button.set_sensitive(true);

            match result {
                Ok(_) => {
                    let msg = if let Some(ref unit) = unit_option {
                        // toast message success
                        format2!(
                            pgettext(
                                "toast",
                                "{} unit <span fgcolor='{0}' font_family='monospace' size='larger'>{}</span> successful"
                            ),
                            blue,
                            &method_name,
                            unit.primary(),
                        )
                    } else {
                        // toast message success (no unit) -- "{ACTION} successful."
                        format2!(pgettext("toast", "{} successful."), &method_name)
                    };
                    control_panel.add_toast_message(&msg, true)
                }
                Err(ref error) => {
                    let red = red(is_dark).get_color();

                    let msg = if let Some(ref unit) = unit_option {
                        format2!(
                            // toast message failed
                            pgettext(
                                "toast",
                                "{} unit <span fgcolor='{}' font_family='monospace' size='larger'>{}</span> failed. Reason: <span fgcolor='{}'>{}</span>."
                            ),
                            &method_name,
                            blue,
                            unit.primary(),
                            red,
                            error.human_error_type()
                        )
                    } else {
                        format2!(
                            // toast message failed (no unit) -- "{ACTION} failed. Reason: <span fgcolor='{CSS}'>{SYSTEMD ERROR (English)}</span>."
                            pgettext("toast", "{} failed. Reason: <span fgcolor='{}'>{}</span>."),
                            &method_name,
                            red,
                            error.human_error_type()
                        )
                    };

                    warn!("{msg} {error:?}");
                    control_panel.add_toast_message(&msg, true);
                }
            }

            return_handle(&method_name, unit_option.as_ref(), result, &control_panel)
        });
    }

    pub(super) fn current_unit(&self) -> Option<UnitInfo> {
        self.current_unit.borrow().clone()
    }

    fn more_action_popover_shown(&self, side_panel: &SideControlPanel) {
        let unit_option = self.current_unit();

        side_panel.more_action_popover_shown(&self.obj(), unit_option);
    }
}

#[glib::derived_properties]
impl ObjectImpl for UnitControlPanelImpl {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_modes(&self.start_modes, UnitContolType::Start);
        self.set_modes(&self.stop_modes, UnitContolType::Stop);
        self.set_modes(&self.restart_modes, UnitContolType::Restart);

        self.unit_panel_stack.connect_pages_notify(|view_stack| {
            info!("page notify {:?}", view_stack.visible_child_name());
        });

        {
            const VISIBLE_FALSE: InterPanelMessage<'_> = InterPanelMessage::PanelVisible(false);
            const VISIBLE_TRUE: InterPanelMessage<'_> = InterPanelMessage::PanelVisible(true);

            let unit_journal_panel = self.unit_journal_panel.clone();
            let unit_dependencies_panel = self.unit_dependencies_panel.clone();
            let unit_file_panel = self.unit_file_panel.clone();
            self.unit_panel_stack
                .connect_visible_child_notify(move |view_stack| {
                    debug!(
                        "connect_visible_child_notify {:?}",
                        view_stack.visible_child_name()
                    );

                    if let Some(child) = view_stack.visible_child() {
                        if child.downcast_ref::<JournalPanel>().is_some() {
                            debug!("It a journal");
                            unit_dependencies_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_journal_panel.set_inter_message(&VISIBLE_TRUE);
                        } else if child.downcast_ref::<UnitDependenciesPanel>().is_some() {
                            debug!("It's  dependency");
                            unit_dependencies_panel.set_inter_message(&VISIBLE_TRUE);
                            unit_file_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_journal_panel.set_inter_message(&VISIBLE_FALSE);
                        } else if child.downcast_ref::<UnitFilePanel>().is_some() {
                            debug!("It's file panel");
                            unit_dependencies_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_message(&VISIBLE_TRUE);
                            unit_journal_panel.set_inter_message(&VISIBLE_FALSE);
                        } else {
                            //It' the last one InfoPanel
                            unit_journal_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_dependencies_panel.set_inter_message(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_message(&VISIBLE_FALSE);
                        }
                    }
                });
        }

        let family = PREFERENCES.font_family();
        let size = PREFERENCES.font_size();

        if !font_management::is_default_font(&family, size) {
            let mut font_description = FontDescription::new();

            if !family.is_empty() {
                font_description.set_family(&family);
            }

            if size != 0 {
                let scaled_size = size as i32 * pango::SCALE;
                font_description.set_size(scaled_size);
            }

            let action = InterPanelMessage::Font(Some(&font_description));

            self.set_inter_message(&action);

            FONT_CONTEXT.set_font_description(font_description);
        }

        let more_action_panel = SideControlPanel::new();

        self.more_action_popover.set_child(Some(&more_action_panel));
        let _ = self.more_action_panel.set(more_action_panel.clone());

        let a = self.obj().clone();
        let more_action_panel = more_action_panel.clone();
        self.more_action_popover.connect_show(move |_popover| {
            info!("More action popover shown");

            a.imp().more_action_popover_shown(&more_action_panel);
        });

        /*         self.show_more_button
        .bind_property::<gtk::Popover>("active", self.side_overlay.as_ref(), "collapsed")
        .bidirectional()
        .invert_boolean()
        .build(); */
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
