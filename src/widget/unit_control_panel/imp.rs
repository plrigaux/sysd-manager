use super::{
    UnitControlPanel, controls, enums::UnitContolType, side_control_panel::SideControlPanel,
};
use crate::{
    consts::{
        ACTION_FIND_IN_TEXT_OPEN, ACTION_FIND_IN_TEXT_TOGGLE, ACTION_WIN_FAVORITE_SET,
        ACTION_WIN_FAVORITE_TOGGLE, ACTION_WIN_REFRESH_POP_MENU, ACTION_WIN_RELOAD_UNIT,
        ACTION_WIN_RESTART_UNIT, ACTION_WIN_START_UNIT, ACTION_WIN_STOP_UNIT,
        ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY, DESTRUCTIVE_ACTION, SETTING_FIND_IN_TEXT_OPEN,
        SUGGESTED_ACTION,
    },
    format2, systemd_gui,
    utils::{
        font_management::{self, FONT_CONTEXT, create_provider},
        palette::{dark_blue, dark_red},
    },
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        journal::JournalPanel,
        preferences::data::{KEY_PREF_CONTROLS_ALWAYS_SHOWS_START_STOP, PREFERENCES},
        set_favorite_info,
        text_search::PanelID,
        unit_dependencies_panel::UnitDependenciesPanel,
        unit_file_panel::UnitFilePanel,
        unit_info::UnitInfoPanel,
    },
};
use adw::{prelude::*, subclass::prelude::*};
use base::enums::UnitDBusLevel;
use gettextrs::pgettext;
use gtk::{
    gio,
    glib::{self},
    pango::{self, FontDescription},
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
use strum::IntoEnumIterator;
use systemd::{
    self, ReStartStop,
    data::UnitInfo,
    enums::{ActiveState, StartStopMode, UnitFileStatus},
    errors::SystemdErrors,
};
use tracing::{debug, error, info, warn};

const INFO_PAGE: &str = "info_page";
const DEPENDENCIES_PAGE: &str = "dependencies_page";
const JOURNAL_PAGE: &str = "journal_page";
const DEFINITION_FILE_PAGE: &str = "definition_file_page";

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
    reload_unit_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    more_action_popover: TemplateChild<gtk::Popover>,

    #[template_child]
    start_modes: TemplateChild<gtk::Box>,

    #[template_child]
    stop_modes: TemplateChild<gtk::Box>,

    #[template_child]
    restart_modes: TemplateChild<gtk::Box>,

    #[template_child]
    reload_unit_modes: TemplateChild<gtk::Box>,

    #[template_child]
    unit_panel_stack: TemplateChild<adw::ViewStack>,

    #[template_child]
    favorite_button: TemplateChild<gtk::Button>,

    app_window: OnceCell<AppWindow>,

    current_unit: RefCell<Option<UnitInfo>>,

    search_bar: RefCell<gtk::SearchBar>,

    #[property(get, set)]
    pub start_mode: RefCell<String>,
    #[property(get, set)]
    pub stop_mode: RefCell<String>,
    #[property(get, set)]
    pub restart_mode: RefCell<String>,
    #[property(get, set)]
    pub reload_unit_mode: RefCell<String>,
    #[property(get, set=Self::set_always_shows_start_stop)]
    pub always_shows_start_stop: Cell<bool>,

    old_font_provider: RefCell<Option<gtk::CssProvider>>,
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
        self.unit_journal_panel.register(app_window);

        /*         if let Some(side_panel) = self.more_action_panel.get() {
            side_panel.set_app_window(app_window);
        } else {
            warn!("Side Panel Should not be None");
        } */

        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");

        let unit_control_panel = self.obj().clone();
        let more_action_panel = SideControlPanel::new(&unit_control_panel);

        self.more_action_popover.set_child(Some(&more_action_panel));

        let more_action_panel = more_action_panel.clone();
        self.more_action_popover.connect_show(move |_popover| {
            more_action_panel.more_action_popover_shown();
        });

        let action_start_unit = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_START_UNIT[4..])
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    cpanel.imp().start_restart_selected_unit(ReStartStop::Start);
                })
                .build()
        };

        let action_stop_unit = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_STOP_UNIT[4..])
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    cpanel.imp().start_restart_selected_unit(ReStartStop::Stop);
                })
                .build()
        };

        let action_restart_unit = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_RESTART_UNIT[4..])
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    cpanel
                        .imp()
                        .start_restart_selected_unit(ReStartStop::Restart);
                })
                .build()
        };

        let action_reload_unit = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_RELOAD_UNIT[4..])
                .activate(move |_application: &AppWindow, _b, _target_value| {
                    cpanel
                        .imp()
                        .start_restart_selected_unit(ReStartStop::ReloadUnit);
                })
                .build()
        };

        let action_favorite_set = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_FAVORITE_SET[4..])
                .activate(move |_application: &AppWindow, _b, _target_value| {})
                .state(false.to_variant())
                .change_state(move |_a, simple_action, new_state| {
                    let Some(state) = simple_action.state().and_then(|v| v.get::<bool>()) else {
                        warn!("no state");
                        return;
                    };

                    let Some(new_state) = new_state.and_then(|v| v.get::<bool>()) else {
                        warn!("no new state");
                        return;
                    };

                    debug!(
                        "Action {ACTION_WIN_FAVORITE_SET} state {state} new state {:?}",
                        new_state
                    );

                    simple_action.set_state(&new_state.to_variant());
                    cpanel.imp().set_favorite(new_state);
                })
                .build()
        };

        let action_unit_has_reload = {
            let cpanel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY[4..])
                .activate(move |_application: &AppWindow, _b, target_value| {
                    let visible = target_value.and_then(|v| v.get::<bool>()).unwrap_or(false);

                    cpanel.imp().reload_unit_button.set_visible(visible);
                })
                .parameter_type(Some(glib::VariantTy::BOOLEAN))
                .build()
        };

        let find_in_text_toogle = {
            let control_panel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_FIND_IN_TEXT_TOGGLE[4..])
                .activate(move |_application: &AppWindow, _, target_value| {
                    let settings = systemd_gui::new_settings();
                    let value = settings.boolean(SETTING_FIND_IN_TEXT_OPEN);
                    if let Err(err) = settings.set_boolean(SETTING_FIND_IN_TEXT_OPEN, !value) {
                        warn!("{SETTING_FIND_IN_TEXT_OPEN} {err}")
                    }

                    if !value {
                        let panel: PanelID = target_value.into();

                        match panel {
                            PanelID::Info => {
                                control_panel.imp().unit_info_panel.focus_text_search()
                            }
                            PanelID::Dependencies => control_panel
                                .imp()
                                .unit_dependencies_panel
                                .focus_text_search(),
                            PanelID::File => {
                                control_panel.imp().unit_file_panel.focus_text_search()
                            }
                            PanelID::Journal => {
                                control_panel.imp().unit_journal_panel.focus_text_search()
                            }
                        }
                    }
                })
                .parameter_type(Some(glib::VariantTy::BYTE))
                .build()
        };

        let find_in_text_open = {
            let control_panel = self.obj().clone();
            gio::ActionEntry::builder(&ACTION_FIND_IN_TEXT_OPEN[4..])
                .activate(move |_application: &AppWindow, _, _| {
                    let settings = systemd_gui::new_settings();
                    if let Err(err) = settings.set_boolean(SETTING_FIND_IN_TEXT_OPEN, true) {
                        warn!("{SETTING_FIND_IN_TEXT_OPEN} {err}")
                    }

                    match control_panel
                        .imp()
                        .unit_panel_stack
                        .visible_child_name()
                        .as_deref()
                    {
                        Some(INFO_PAGE) => control_panel.imp().unit_info_panel.focus_text_search(),
                        Some(DEPENDENCIES_PAGE) => control_panel
                            .imp()
                            .unit_dependencies_panel
                            .focus_text_search(),
                        Some(DEFINITION_FILE_PAGE) => {
                            control_panel.imp().unit_file_panel.focus_text_search()
                        }
                        Some(JOURNAL_PAGE) => {
                            control_panel.imp().unit_journal_panel.focus_text_search()
                        }
                        _ => {}
                    }
                })
                .build()
        };

        app_window.add_action_entries([
            action_start_unit,
            action_stop_unit,
            action_restart_unit,
            action_reload_unit,
            action_favorite_set,
            action_unit_has_reload,
            find_in_text_toogle,
            find_in_text_open,
        ]);

        //Disable buttons
        let app_window = app_window.clone();
        glib::spawn_future_local(async move {
            app_window.action_set_enabled(ACTION_WIN_START_UNIT, false);
            app_window.action_set_enabled(ACTION_WIN_STOP_UNIT, false);
            app_window.action_set_enabled(ACTION_WIN_RESTART_UNIT, false);
            app_window.action_set_enabled(ACTION_WIN_RELOAD_UNIT, false);
            app_window.action_set_enabled(ACTION_WIN_FAVORITE_TOGGLE, false);
        });
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
            UnitFileStatus::Enabled
        } else {
            UnitFileStatus::Disabled
        };

        controls::switch_ablement_state_set(
            &self.obj(),
            expected_new_status,
            switch,
            &unit,
            Rc::new(Box::new(|| {})),
        );

        self.unit_info_panel
            .set_inter_message(&InterPanelMessage::UnitChange(Some(&unit)));
        true // to stop the signal emission
    }

    #[template_callback]
    fn button_search_toggled(&self, toggle_button: &gtk::ToggleButton) {
        self.search_bar
            .borrow()
            .set_search_mode(toggle_button.is_active());
    }
}

impl UnitControlPanelImpl {
    fn start_restart_selected_unit(&self, re_start_stop: ReStartStop) {
        let unit = current_unit!(self);
        let Some(app_window) = self.app_window.get() else {
            error!("No AppWindow ");
            return;
        };

        let (action_name, action, start_mode): (&str, UnitContolType, StartStopMode) =
            match re_start_stop {
                ReStartStop::Start => (
                    ACTION_WIN_START_UNIT,
                    UnitContolType::Start,
                    (&self.start_mode).into(),
                ),
                ReStartStop::Stop => (
                    ACTION_WIN_STOP_UNIT,
                    UnitContolType::Stop,
                    (&self.stop_mode).into(),
                ),
                ReStartStop::Restart => (
                    ACTION_WIN_RESTART_UNIT,
                    UnitContolType::Restart,
                    (&self.restart_mode).into(),
                ),
                ReStartStop::ReloadUnit => (
                    ACTION_WIN_RELOAD_UNIT,
                    UnitContolType::Reload,
                    (&self.reload_unit_mode).into(),
                ),
            };

        let unit_control_panel = self.obj().clone();

        let primary_name = unit.primary();
        let level = unit.dbus_level();
        let app_window = app_window.clone();

        glib::spawn_future_local(async move {
            app_window.action_set_enabled(action_name, false);

            let (sender, receiver) = tokio::sync::oneshot::channel();
            systemd::runtime().spawn(async move {
                let response =
                    systemd::restartstop_unit(level, &primary_name, start_mode, re_start_stop)
                        .await;
                if let Err(e) = sender.send(response) {
                    error!("Channel closed unexpectedly: {e:?}");
                }
            });

            let Ok(start_results) = receiver
                .await
                .inspect_err(|err| error!("Tokio channel dropped {err:?}"))
            else {
                return;
            };

            unit_control_panel.imp().start_restart(
                &unit.primary(),
                Some(&unit),
                start_results,
                action,
                start_mode,
            );

            app_window.action_set_enabled(action_name, true);
            if let Err(activate) =
                unit_control_panel.activate_action(ACTION_WIN_REFRESH_POP_MENU, None)
            {
                warn!("Fail action activation {activate:?}");
            }
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
        match start_results {
            Ok(_job) => {
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
                    pgettext("toast", "Unit {} has been <{0}>{}</{0}> with the mode {}"),
                    red_green,
                    format!("<unit>{}</unit>", unit_name),
                    action.past_participle(),
                    format!("<unit>{}</unit>", mode.as_str())
                );

                self.add_toast_message(&info, true);

                if let Some(unit) = unit_op {
                    unit.set_active_state(action.on_succes_unit_state());
                    self.highlight_controls(unit);
                }

                self.unit_info_panel
                    .set_inter_message(&InterPanelMessage::Refresh(unit_op));
            }
            Err(err) => {
                warn!("{} FAILED, Unit {:?} {:?}", action.code(), unit_name, err);

                let info = format2!(
                    //toast message error --  "Can't {ACTION} the unit <unit>{UNITNAME}</unit>, because: {SYSTEMD HUMAN ERROR (english)}"),
                    pgettext("toast", "Can't {} the unit {}, because: {}"),
                    action.label(),
                    format!("<unit>{}</unit>", unit_name),
                    err.human_error_type()
                );

                self.add_toast_message(&info, true);
            }
        };
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

        controls::handle_switch_sensivity(&self.ablement_switch, unit, true);

        let Some(app_window) = self.app_window.get() else {
            error!("No AppWindow ");
            return;
        };

        app_window.action_set_enabled(ACTION_WIN_START_UNIT, true);
        app_window.action_set_enabled(ACTION_WIN_STOP_UNIT, true);
        app_window.action_set_enabled(ACTION_WIN_RESTART_UNIT, true);
        app_window.action_set_enabled(ACTION_WIN_FAVORITE_TOGGLE, true);
        self.restart_button.set_sensitive(true);
        //self.kill_button.set_sensitive(true);

        self.highlight_controls(unit);

        if let Some(fav) = app_window.action_state(&ACTION_WIN_FAVORITE_SET[4..])
            && let Some(is_favorite) = fav.get::<bool>()
        {
            self.set_favorite(is_favorite);
        }
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
            InterPanelMessage::JournalFilterBoot(_) => {
                self.display_journal_page();
                self.forward_inter_actions(action)
            }

            InterPanelMessage::EnableUnit(unit, call_back) => {
                controls::switch_ablement_state_set(
                    &self.obj(),
                    UnitFileStatus::Enabled,
                    &self.ablement_switch,
                    unit,
                    call_back.clone(),
                );
            }
            InterPanelMessage::DisableUnit(unit, call_back) => {
                controls::switch_ablement_state_set(
                    &self.obj(),
                    UnitFileStatus::Disabled,
                    &self.ablement_switch,
                    unit,
                    call_back.clone(),
                );
            }

            InterPanelMessage::ReenableUnit(unit, call_back) => {
                controls::reenable_unit(
                    &self.obj(),
                    &self.ablement_switch,
                    unit,
                    call_back.clone(),
                );
            }

            InterPanelMessage::Refresh(unit) => {
                if let Some(unit) = unit.as_deref() {
                    let unit = unit.clone();
                    self.current_unit.replace(Some(unit));
                }

                let binding = self.current_unit.borrow();
                if let Some(unit) = binding.as_ref() {
                    self.highlight_controls(unit);
                }
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

                if !self.always_shows_start_stop.get() {
                    self.start_button.set_visible(false);
                    self.restart_button.set_visible(true);
                    self.stop_button.set_visible(true);
                }
            }
            ActiveState::Inactive | ActiveState::Deactivating => {
                self.stop_button.remove_css_class(DESTRUCTIVE_ACTION);
                self.start_button.add_css_class(SUGGESTED_ACTION);

                if !self.always_shows_start_stop.get() {
                    self.start_button.set_visible(true);
                    self.restart_button.set_visible(false);
                    self.stop_button.set_visible(false);
                }
            }
            _ => {
                self.stop_button.remove_css_class(DESTRUCTIVE_ACTION);
                self.start_button.remove_css_class(SUGGESTED_ACTION);

                if !self.always_shows_start_stop.get() {
                    self.start_button.set_visible(true);
                    self.restart_button.set_visible(true);
                    self.stop_button.set_visible(true);
                }
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
        self.unit_panel_stack.set_visible_child_name(INFO_PAGE);
    }

    pub(super) fn display_dependencies_page(&self) {
        self.unit_panel_stack
            .set_visible_child_name(DEPENDENCIES_PAGE);
    }

    pub(super) fn display_journal_page(&self) {
        self.unit_panel_stack.set_visible_child_name(JOURNAL_PAGE);
    }

    pub fn display_definition_file_page(&self) {
        self.unit_panel_stack
            .set_visible_child_name(DEFINITION_FILE_PAGE);
    }

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
        let blue = dark_blue().get_color();

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
                    let red = dark_red().get_color();

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

    pub fn set_favorite(&self, is_favorite: bool) {
        let unit = self.current_unit.borrow();
        let (favorite_icon, tooltip) = set_favorite_info(is_favorite, &unit);

        self.favorite_button.set_icon_name(favorite_icon);
        self.favorite_button.set_tooltip_markup(Some(&tooltip));
    }

    fn set_always_shows_start_stop(&self, value: bool) {
        self.always_shows_start_stop.set(value);

        if self.always_shows_start_stop.get() {
            self.start_button.set_visible(true);
            self.stop_button.set_visible(true);
        } else if let Some(unit) = self.current_unit.borrow().as_ref() {
            self.highlight_controls(unit);
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
        self.set_modes(&self.reload_unit_modes, UnitContolType::Reload);

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

        let settings = systemd_gui::new_settings();

        settings
            .bind::<UnitControlPanel>(
                KEY_PREF_CONTROLS_ALWAYS_SHOWS_START_STOP,
                &self.obj(),
                "always-shows-start-stop",
            )
            .build();
    }
}

impl WidgetImpl for UnitControlPanelImpl {}
impl BoxImpl for UnitControlPanelImpl {}
