use std::cell::{Cell, OnceCell, RefCell};

use adw::{subclass::prelude::*, Toast};
use gtk::{
    gio,
    glib::{self},
    pango::{self, FontDescription},
    prelude::*,
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
        font_management::{self, create_provider, FONT_CONTEXT},
        writer::UnitInfoWriter,
    },
    widget::{
        app_window::AppWindow, journal::JournalPanel, kill_panel::KillPanel,
        preferences::data::PREFERENCES, unit_dependencies_panel::UnitDependenciesPanel,
        unit_file_panel::UnitFilePanel, unit_info::UnitInfoPanel, InterPanelAction,
    },
};

use super::{controls, enums::UnitContolType, UnitControlPanel};
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
    show_more_button: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    restart_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    side_overlay: TemplateChild<adw::OverlaySplitView>,

    /*     #[template_child]
    kill_panel: TemplateChild<KillPanel>, */
    #[template_child]
    start_modes: TemplateChild<gtk::Box>,

    #[template_child]
    stop_modes: TemplateChild<gtk::Box>,

    #[template_child]
    restart_modes: TemplateChild<gtk::Box>,

    #[template_child]
    unit_panel_stack: TemplateChild<adw::ViewStack>,

    toast_overlay: OnceCell<adw::ToastOverlay>,

    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,

    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,

    old_font_provider: RefCell<Option<gtk::CssProvider>>,

    kill_signal_window: RefCell<Option<KillPanel>>,
    queue_signal_window: RefCell<Option<KillPanel>>,

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
    ($app:expr) => {{
        current_unit!($app, ())
    }};

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
    pub(super) fn set_overlay(&self, app_window: &AppWindow, toast_overlay: &adw::ToastOverlay) {
        //self.kill_panel.register(&self.side_overlay, toast_overlay);
        self.unit_file_panel.register(app_window, toast_overlay);
        self.unit_dependencies_panel.register(app_window);
        self.unit_info_panel.register(app_window);

        if let Err(e) = self.toast_overlay.set(toast_overlay.clone()) {
            warn!("Set Toast Overlay Issue: {:?}", e)
        }
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
            self.toast_overlay.get().unwrap(),
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
    fn show_more_button_clicked(&self, _button: &gtk::Button) {
        //let unit = current_unit!(self);

        //self.kill_panel.set_unit(Some(&unit));

        /*         let collapsed = self.side_overlay.is_collapsed();
        self.side_overlay.set_collapsed(!collapsed); */
    }

    #[template_callback]
    fn sidebar_close_button_clicked(&self, _button: &gtk::Button) {
        //let unit = current_unit!(self);

        //self.kill_panel.set_unit(Some(&unit));

        self.side_overlay.set_collapsed(true);
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.search_bar
            .borrow()
            .set_search_mode(toggle_button.is_active());
    }

    #[template_callback]
    fn kill_button_clicked(&self, _button: &gtk::Button) {
        self.kill_or_queue_new_window(&self.kill_signal_window, KillPanel::new_kill_window);
    }

    #[template_callback]
    fn send_signal_button_clicked(&self, _button: &gtk::Button) {
        self.kill_or_queue_new_window(&self.queue_signal_window, KillPanel::new_signal_window);
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

            unit_control_panel.start_restart(
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
                let is_dark = self.is_dark.get();
                let blue = if is_dark {
                    UnitInfoWriter::blue_dark()
                } else {
                    UnitInfoWriter::blue_light()
                };

                let red_green = controls::red_green(action != UnitContolType::Stop, is_dark);

                let info = format!(
                    "Unit <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span> has been <span fgcolor='{red_green}'>{}</span> with the mode <span fgcolor='{blue}' font_family='monospace'>{}</span>",
                    unit.primary(),
                    action.past_participle(),
                    mode.as_str()
                );
                info!("{info}");

                let toast = Toast::builder().title(&info).use_markup(true).build();
                self.toast_overlay.get().unwrap().add_toast(toast);

                unit.set_active_state(expected_active_state);
                self.highlight_controls(unit);

                Some(job)
            }
            Err(e) => {
                let is_dark = self.is_dark.get();
                let blue = if is_dark {
                    UnitInfoWriter::blue_dark()
                } else {
                    UnitInfoWriter::blue_light()
                };

                let info = format!(
                    "Can't {} the unit <span fgcolor='{blue}' font_family='monospace' size='larger'>{}</span>, because: {:?}",
                    action.as_str(),
                    unit.primary(),
                    e
                );

                warn!("{info}");

                let toast = Toast::builder().title(&info).use_markup(true).build();
                self.toast_overlay.get().unwrap().add_toast(toast);

                None
            }
        };

        let Some(_job) = job_op else {
            return;
        };

        self.unit_info_panel.display_unit_info(Some(unit));
    }

    pub(super) fn selection_change(&self, unit: Option<&UnitInfo>) {
        self.unit_info_panel.display_unit_info(unit);
        self.unit_file_panel.set_unit(unit);
        self.unit_journal_panel.set_unit(unit);
        //self.kill_panel.set_unit(unit);
        self.unit_dependencies_panel.set_unit(unit);

        let kill_signal_window = self.kill_signal_window.borrow();
        if let Some(kill_signal_window) = kill_signal_window.as_ref() {
            kill_signal_window.set_unit(unit);
        }

        let send_signal_window = self.queue_signal_window.borrow();
        if let Some(send_signal_window) = send_signal_window.as_ref() {
            send_signal_window.set_unit(unit);
        }

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
                self.highlight_controls(unit);
                return;
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

    pub fn set_inter_action(&self, action: &InterPanelAction) {
        match *action {
            InterPanelAction::Font(font_description) => {
                let provider = create_provider(&font_description);
                {
                    let binding = self.old_font_provider.borrow();
                    let old_provider = binding.as_ref();
                    let new_action =
                        InterPanelAction::FontProvider(old_provider, provider.as_ref());
                    self.forward_inter_actions(&new_action);
                }
                self.old_font_provider.replace(provider);
            }
            InterPanelAction::IsDark(is_dark) => {
                self.set_dark(is_dark);
                self.forward_inter_actions(action)
            }
            _ => self.forward_inter_actions(action),
        }
    }

    fn forward_inter_actions(&self, action: &InterPanelAction) {
        self.unit_info_panel.set_inter_action(action);
        self.unit_dependencies_panel.set_inter_action(action);
        self.unit_file_panel.set_inter_action(action);
        self.unit_journal_panel.set_inter_action(action);

        let kill_signal_window = self.kill_signal_window.borrow();
        if let Some(kill_signal_window) = kill_signal_window.as_ref() {
            kill_signal_window.set_inter_action(action);
        }

        let send_signal_window = self.queue_signal_window.borrow();
        if let Some(send_signal_window) = send_signal_window.as_ref() {
            send_signal_window.set_inter_action(action);
        }
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

    pub(super) fn toast_overlay(&self) -> Option<&adw::ToastOverlay> {
        self.toast_overlay.get()
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

    fn kill_or_queue_new_window(
        &self,
        window_cell: &RefCell<Option<KillPanel>>,
        new_kill_window_fn: fn(Option<&UnitInfo>, bool, &UnitControlPanel) -> KillPanel,
    ) {
        let binding = self.current_unit.borrow();
        let create_new = {
            let kill_signal_window = window_cell.borrow();
            if let Some(kill_signal_window) = kill_signal_window.as_ref() {
                kill_signal_window.set_unit(binding.as_ref());
                kill_signal_window.set_inter_action(&InterPanelAction::IsDark(self.is_dark.get()));
                kill_signal_window.present();
                false
            } else {
                true
            }
        };

        if create_new {
            let kill_signal_window =
                new_kill_window_fn(binding.as_ref(), self.is_dark.get(), &self.obj());
            kill_signal_window.present();

            window_cell.replace(Some(kill_signal_window));
        }
    }

    pub fn unlink_child(&self, is_signal: bool) {
        if is_signal {
            self.queue_signal_window.replace(None);
        } else {
            self.kill_signal_window.replace(None);
        }
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
            const VISIBLE_FALSE: InterPanelAction<'_> = InterPanelAction::PanelVisible(false);
            const VISIBLE_TRUE: InterPanelAction<'_> = InterPanelAction::PanelVisible(true);

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
                            unit_dependencies_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_journal_panel.set_inter_action(&VISIBLE_TRUE);
                        } else if child.downcast_ref::<UnitDependenciesPanel>().is_some() {
                            debug!("It's  dependency");
                            unit_dependencies_panel.set_inter_action(&VISIBLE_TRUE);
                            unit_file_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_journal_panel.set_inter_action(&VISIBLE_FALSE);
                        } else if child.downcast_ref::<UnitFilePanel>().is_some() {
                            debug!("It's file panel");
                            unit_dependencies_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_action(&VISIBLE_TRUE);
                            unit_journal_panel.set_inter_action(&VISIBLE_FALSE);
                        } else {
                            //It' the last one InfoPanel
                            unit_journal_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_dependencies_panel.set_inter_action(&VISIBLE_FALSE);
                            unit_file_panel.set_inter_action(&VISIBLE_FALSE);
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

            let action = InterPanelAction::Font(Some(&font_description));

            self.set_inter_action(&action);

            FONT_CONTEXT.set_font_description(font_description);
        }

        self.show_more_button
            .bind_property::<adw::OverlaySplitView>(
                "active",
                self.side_overlay.as_ref(),
                "collapsed",
            )
            .bidirectional()
            .transform_to(|_binding, is_active: bool| Some(!is_active))
            .transform_from(|_binding, is_active: bool| Some(!is_active))
            .build();
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
