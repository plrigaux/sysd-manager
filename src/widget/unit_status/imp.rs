use super::construct_info::fill_all_info;
use crate::{
    consts::{ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY, SETTING_FIND_IN_TEXT, *},
    systemd::data::UnitInfo,
    systemd_gui::{self, new_settings},
    utils::{
        font_management::set_text_view_font,
        text_view_hyperlink::{self, LinkActivator},
        writer::UnitInfoWriter,
    },
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::data::KEY_PREF_UNIT_DESCRIPTION_WRAP,
        text_search::{self, TextSearchEntry},
    },
};
use gettextrs::pgettext;
use gtk::{
    TemplateChild,
    glib::{self},
    prelude::*,
    subclass::{
        box_::BoxImpl,
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
        },
    },
};
use std::cell::OnceCell;
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, warn};
use zvariant::Value;

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_status_panel.ui")]
#[properties(wrapper_type = super::UnitStatusPanel)]
pub struct UnitStatusPanelImp {
    #[template_child]
    show_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    pub(super) unit_status_textview: TemplateChild<gtk::TextView>,

    #[template_child]
    find_text_button: TemplateChild<gtk::ToggleButton>,

    unit: RefCell<Option<UnitInfo>>,

    #[property(name="wrap-word", get=Self::get_wrap_word,set=Self::set_wrap_word, type = bool)]
    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,

    app_window: OnceCell<AppWindow>,

    text_search_entry: OnceCell<TextSearchEntry>,
}

#[gtk::template_callbacks]
impl UnitStatusPanelImp {
    #[template_callback]
    fn refresh_info_clicked(&self, _button: &gtk::Button) {
        self.refresh_panels(None);
    }
}

impl UnitStatusPanelImp {
    //FIXME It's been called twice
    fn set_unit(&self, unit: Option<&UnitInfo>) {
        match unit {
            Some(unit) => {
                let old_unit = self.unit.replace(Some(unit.clone()));
                if !unit.equals_op(old_unit.as_ref()) {
                    self.update_unit_status_panel(unit)
                }
            }
            None => {
                self.unit.replace(None);
                self.clear();
            }
        };

        self.set_sensitivity();
    }

    fn set_sensitivity(&self) {
        if self.unit.borrow().is_some() {
            self.show_all_button.set_sensitive(true);
            self.refresh_button.set_sensitive(true);
        } else {
            self.show_all_button.set_sensitive(false);
            self.refresh_button.set_sensitive(false);
        }
    }

    /// Updates the associated journal `TextView` with the contents of the unit's journal log.
    fn update_unit_status_panel(&self, unit: &UnitInfo) {
        let buf = self.clear();
        let start_iter = buf.start_iter();

        let mut info_writer = UnitInfoWriter::new(buf, start_iter);

        let map = fill_all_info(unit, &mut info_writer);

        self.on_new_text();

        let has_reload_unit_capabilities = if let Some(value) = map.get("ExecReload")
            && let Value::Array(array) = value as &Value
            && !array.is_empty()
        {
            true
        } else {
            false
        };

        if let Err(err) = self.unit_status_textview.activate_action(
            ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY,
            Some(&has_reload_unit_capabilities.to_variant()),
        ) {
            warn!(
                "Error {} activating action {}",
                err, ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY
            );
        }

        if let Some(app_window) = self.app_window.get() {
            app_window.action_set_enabled(
                ACTION_WIN_RELOAD_UNIT,
                unit.is_active() && has_reload_unit_capabilities,
            );
        }
    }

    pub fn on_new_text(&self) {
        // if !search_bar.is_search_mode() {
        //     return;
        // }

        if let Some(text_search_bar) = self.text_search_entry.get() {
            text_search_bar.find_text();
        }
    }

    fn clear(&self) -> gtk::TextBuffer {
        let unit_info_text_view: &gtk::TextView = self.unit_status_textview.as_ref();

        let buf = unit_info_text_view.buffer();

        buf.set_text(""); // clear text
        buf
    }

    pub(super) fn register(&self, app_window: &AppWindow) {
        let activator = LinkActivator::new(Some(app_window.clone()));

        text_view_hyperlink::build_textview_link_platform(
            &self.unit_status_textview,
            self.hovering_over_link_tag.clone(),
            activator,
        );

        if self.app_window.set(app_window.clone()).is_err() {
            warn!("Set only once");
        }

        let settings = systemd_gui::new_settings();

        let action = settings.create_action(KEY_PREF_UNIT_DESCRIPTION_WRAP);

        app_window.add_action(&action);

        let wrap = settings.boolean(KEY_PREF_UNIT_DESCRIPTION_WRAP);
        self.set_wrap_word(wrap);
    }

    pub(super) fn refresh_panels(&self, unit: Option<&UnitInfo>) {
        if let Some(unit) = unit {
            self.unit.replace(Some(unit.clone()));
        }

        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        self.update_unit_status_panel(unit)
    }

    pub(super) fn set_inter_message(&self, action: &InterPanelMessage) {
        match *action {
            InterPanelMessage::FontProvider(old, new) => {
                set_text_view_font(old, new, &self.unit_status_textview);
            }
            InterPanelMessage::UnitChange(unit) => self.set_unit(unit),
            InterPanelMessage::Refresh(unit) => self.refresh_panels(unit),
            InterPanelMessage::IsDark(_) => self.refresh_panels(None),
            InterPanelMessage::PanelVisible(visible) => self.set_visible_on_page(visible),
            _ => {}
        }
    }

    fn set_visible_on_page(&self, visible: bool) {
        debug!("set_visible_on_page val {visible}");

        if visible && let Some(text_search_entry) = self.text_search_entry.get() {
            text_search_entry.set_text_view(&self.unit_status_textview);
            text_search_entry.find_text();
        }
    }

    fn get_wrap_word(&self) -> bool {
        self.unit_status_textview.wrap_mode() != gtk::WrapMode::None
    }

    fn set_wrap_word(&self, wrap: bool) {
        let wrap_mode = if wrap {
            gtk::WrapMode::Word
        } else {
            gtk::WrapMode::None
        };
        self.unit_status_textview.set_wrap_mode(wrap_mode);
    }

    pub fn set_text_search_entry(&self, text_search_entry: &TextSearchEntry) {
        let _ = self.text_search_entry.set(text_search_entry.clone());

        text_search_entry.set_text_view(self.unit_status_textview.as_ref());
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitStatusPanelImp {
    const NAME: &'static str = "UnitStatusPanel";
    type Type = super::UnitStatusPanel;
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
impl ObjectImpl for UnitStatusPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_sensitivity();

        let settings = new_settings();

        settings
            .bind(
                KEY_PREF_UNIT_DESCRIPTION_WRAP,
                self.obj().as_ref(),
                "wrap-word",
            )
            .build();

        let menu = gio::Menu::new();
        let section_menu = gio::Menu::new();
        text_search::create_menu_item(&section_menu);

        //Menu item label for status menu
        let menu_label = pgettext("menu", "Wrap Word");
        let wrap_word_toggle_menu = gio::MenuItem::new(Some(&menu_label), None);
        let action_name = String::from("win.") + KEY_PREF_UNIT_DESCRIPTION_WRAP;
        wrap_word_toggle_menu.set_action_and_target_value(Some(&action_name), None);
        section_menu.append_item(&wrap_word_toggle_menu);

        menu.append_section(None, &section_menu);
        self.unit_status_textview.set_extra_menu(Some(&menu));

        settings
            .bind(
                &SETTING_FIND_IN_TEXT[4..],
                &self.find_text_button.get(),
                "active",
            )
            .build();
    }
}

impl WidgetImpl for UnitStatusPanelImp {}
impl BoxImpl for UnitStatusPanelImp {}
