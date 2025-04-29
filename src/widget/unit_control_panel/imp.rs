use std::cell::{Cell, OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio,
    glib::{self},
    pango::{self, FontDescription},
};
use log::{debug, info, warn};

use crate::{
    consts::{DESTRUCTIVE_ACTION, SUGGESTED_ACTION},
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

use super::{
    UnitControlPanel, controls, enums::UnitContolType, side_control_panel::SideControlPanel,
};
use strum::IntoEnumIterator;

const TTT_HIDE: &str = "Hide sidebar";
const TTT_SHOW: &str = "Show sidebar";

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
    show_more_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    restart_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    side_overlay: TemplateChild<adw::OverlaySplitView>,

    #[template_child]
    start_modes: TemplateChild<gtk::Box>,

    #[template_child]
    stop_modes: TemplateChild<gtk::Box>,

    #[template_child]
    restart_modes: TemplateChild<gtk::Box>,

    #[template_child]
    unit_panel_stack: TemplateChild<adw::ViewStack>,

    app_window: OnceCell<AppWindow>,
    side_panel: OnceCell<SideControlPanel>,

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

        if let Some(side_panel) = self.side_panel.get() {
            side_panel.set_app_window(app_window);
        } else {
            warn!("Side Panel Should not be None");
        }

        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");
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
        );

        self.unit_info_panel.display_unit_info(Some(&unit));
        true // to stop the signal emission
    }

    #[template_callback]
    fn button_start_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::start_unit,
            UnitContolType::Start,
            ActiveState::Active,
        );
    }

    #[template_callback]
    fn button_stop_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::stop_unit,
            UnitContolType::Stop,
            ActiveState::Inactive,
        );
    }

    #[template_callback]
    fn button_restart_clicked(&self, button: &adw::SplitButton) {
        self.start_restart_action(
            button,
            systemd::restart_unit,
            UnitContolType::Restart,
            ActiveState::Active,
        );
    }

    #[template_callback]
    fn show_more_button_clicked(&self, show_more_button: &gtk::ToggleButton) {
        let tooltip_text = if show_more_button.is_active() {
            TTT_HIDE
        } else {
            TTT_SHOW
        };

        show_more_button.set_tooltip_text(Some(tooltip_text));
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
        button: &adw::SplitButton,
        systemd_method: fn(&UnitInfo, StartStopMode) -> Result<String, SystemdErrors>,
        action: UnitContolType,
        expected_active_state: ActiveState,
    ) {
        let unit = current_unit!(self);

        let start_mode: StartStopMode = (&self.start_mode).into();

        let unit_control_panel = self.obj().clone();

        let button = button.clone();
        glib::spawn_future_local(async move {
            button.set_sensitive(false);

            let unit_ = unit.clone();
            let start_results = gio::spawn_blocking(move || systemd_method(&unit_, start_mode))
                .await
                .expect("Task needs to finish successfully.");

            button.set_sensitive(true);

            unit_control_panel.imp().start_restart(
                &unit,
                start_results,
                action,
                expected_active_state,
                start_mode,
            );
        });
    }

    pub(super) fn start_restart(
        &self,
        unit: &UnitInfo,
        start_results: Result<String, SystemdErrors>,
        action: UnitContolType,
        expected_active_state: ActiveState,
        mode: StartStopMode,
    ) {
        let job_op = match start_results {
            Ok(job) => {
                let red_green = if action != UnitContolType::Stop {
                    "green"
                } else {
                    "red"
                };

                let info = format!(
                    "Unit <unit>{}</unit> has been <{red_green}>{}</{red_green}> with the mode <unit>{}</unit>",
                    unit.primary(),
                    action.past_participle(),
                    mode.as_str()
                );
                info!("{info}");

                self.add_toast_message(&info, true);

                unit.set_active_state(expected_active_state);
                self.highlight_controls(unit);

                Some(job)
            }
            Err(err) => {
                let info = format!(
                    "Can't {} the unit <unit>{}</unit>, because: {}",
                    action.as_str(),
                    unit.primary(),
                    err.human_error_type()
                );

                warn!("{info} {:?}", err);

                self.add_toast_message(&info, true);

                None
            }
        };

        let Some(_job) = job_op else {
            return;
        };

        self.unit_info_panel.display_unit_info(Some(unit));
    }

    pub(super) fn selection_change(&self, unit: Option<&UnitInfo>) {
        let action = InterPanelMessage::UnitChange(unit);
        self.set_inter_message(&action);
        self.unit_info_panel.display_unit_info(unit);
        self.unit_file_panel.set_unit(unit);
        self.unit_journal_panel.set_unit(unit);
        self.unit_dependencies_panel.set_unit(unit);

        let unit = match unit {
            Some(u) => u,
            None => {
                self.current_unit.replace(None);
                return;
            }
        };

        let old_unit = self.current_unit.replace(Some(unit.clone()));
        if let Some(old_unit) = old_unit {
            if old_unit.primary() == unit.primary() {
                info! {"Same unit {}", unit.primary() };
                /*                 self.highlight_controls(unit);
                return; */
            }
        }

        controls::handle_switch_sensivity(&self.ablement_switch, unit, true);

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
        match *action {
            InterPanelMessage::Font(font_description) => {
                let provider = create_provider(&font_description);
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
                self.set_dark(is_dark);
                self.forward_inter_actions(action)
            }
            InterPanelMessage::JournalFilterBoot(_) => {
                self.display_journal_page();
                self.forward_inter_actions(action)
            }
            _ => self.forward_inter_actions(action),
        }
    }

    fn forward_inter_actions(&self, action: &InterPanelMessage) {
        self.unit_info_panel.set_inter_message(action);
        self.unit_dependencies_panel.set_inter_message(action);
        self.unit_file_panel.set_inter_message(action);
        self.unit_journal_panel.set_inter_message(action);

        let Some(side_panel) = self.side_panel.get() else {
            warn!("Side Panel Should not be None");
            return;
        };

        side_panel.set_inter_message(action);
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

            let source_property = format!("{}_mode", control_type.as_str());
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

    pub fn unlink_child(&self, is_signal: bool) {
        let Some(side_panel) = self.side_panel.get() else {
            warn!("Side Panel Should not be None");
            return;
        };
        side_panel.unlink_child(is_signal);
    }

    pub(super) fn add_toast_message(&self, message: &str, use_markup: bool) {
        if let Some(app_window) = self.app_window.get() {
            app_window.add_toast_message(message, use_markup);
        }
    }

    pub(super) fn call_method(
        &self,
        method_name: &str,
        button: &impl IsA<gtk::Widget>,
        systemd_method: impl Fn(&UnitInfo) -> Result<(), SystemdErrors> + std::marker::Send + 'static,
        return_handle: impl Fn(&UnitInfo, Result<(), SystemdErrors>, &UnitControlPanel) + 'static,
    ) {
        let binding = self.current_unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("No Unit");
            return;
        };

        let is_dark = true; //self.is_dark.get();
        let blue = blue(is_dark).get_color();

        let control_panel = self.obj().clone();
        let unit = unit.clone();
        let button = button.clone();
        let method_name = method_name.to_owned();

        //   let systemd_method = systemd_method.clone();
        glib::spawn_future_local(async move {
            button.set_sensitive(false);

            let unit2 = unit.clone();
            let result = gio::spawn_blocking(move || systemd_method(&unit2))
                .await
                .expect("Call needs to finish successfully.");

            button.set_sensitive(true);

            match result {
                Ok(_) => {
                    let msg = format!(
                        "{method_name} unit <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> successful.",
                        unit.primary(),
                    );
                    control_panel.add_toast_message(&msg, true)
                }
                Err(ref error) => {
                    let red = red(is_dark).get_color();
                    let msg = format!(
                        "{method_name} unit <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> failed. Reason: <span fgcolor='{red}'>{}</span>.",
                        unit.primary(),
                        error.human_error_type()
                    );

                    warn!("{msg} {:?}", error);
                    control_panel.add_toast_message(&msg, true)
                }
            }

            return_handle(&unit, result, &control_panel)
        });
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

        let sidebar = SideControlPanel::new(&self.obj());

        self.side_overlay.set_sidebar(Some(&sidebar));
        let _ = self.side_panel.set(sidebar);

        self.show_more_button
            .bind_property::<adw::OverlaySplitView>(
                "active",
                self.side_overlay.as_ref(),
                "collapsed",
            )
            .bidirectional()
            .invert_boolean()
            .build();

        self.show_more_button.set_tooltip_text(Some(TTT_SHOW));
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
