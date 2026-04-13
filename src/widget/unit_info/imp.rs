use super::construct_info::fill_all_info;
use crate::{
    consts::{ACTION_FIND_IN_TEXT, ACTION_WIN_UNIT_HAS_RELOAD_UNIT_CAPABILITY, *},
    systemd::data::UnitInfo,
    systemd_gui::new_settings,
    utils::{
        font_management::{set_font_context, set_text_view_font},
        text_view_hyperlink::{self, LinkActivator},
        writer::UnitInfoWriter,
    },
    widget::{
        InterPanelMessage,
        app_window::AppWindow,
        preferences::data::KEY_PREF_UNIT_DESCRIPTION_WRAP,
        text_search::{self, on_new_text, text_search_construct},
    },
};
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
use tracing::{info, warn};
use zvariant::Value;

#[derive(Default, glib::Properties, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
#[properties(wrapper_type = super::UnitInfoPanel)]
pub struct UnitInfoPanelImp {
    #[template_child]
    show_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_info_textview: TemplateChild<gtk::TextView>,

    #[template_child]
    text_search_bar: TemplateChild<gtk::SearchBar>,

    #[template_child]
    find_text_button: TemplateChild<gtk::ToggleButton>,

    unit: RefCell<Option<UnitInfo>>,

    #[property(name="wrap", get=Self::get_wrap,set=Self::set_wrap, type = bool)]
    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,

    app_window: OnceCell<AppWindow>,
}

#[gtk::template_callbacks]
impl UnitInfoPanelImp {
    #[template_callback]
    fn refresh_info_clicked(&self, button: &gtk::Button) {
        info!("button {button:?}");

        self.refresh_panels(None);
    }
}

impl UnitInfoPanelImp {
    //FIXME It's been called twice
    fn set_unit(&self, unit: Option<&UnitInfo>) {
        match unit {
            Some(unit) => {
                let old_unit = self.unit.replace(Some(unit.clone()));
                if !unit.equals_op(old_unit.as_ref()) {
                    self.update_unit_info(unit)
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
    fn update_unit_info(&self, unit: &UnitInfo) {
        let buf = self.clear();
        let start_iter = buf.start_iter();

        let mut info_writer = UnitInfoWriter::new(buf, start_iter);

        let map = fill_all_info(unit, &mut info_writer);

        on_new_text(&self.text_search_bar);

        let has_reload_unit_capabilities = if let Some(value) = map.get("ExecReload")
            && let Value::Array(array) = value as &Value
            && !array.is_empty()
        {
            true
        } else {
            false
        };

        if let Err(err) = self.unit_info_textview.activate_action(
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

    fn clear(&self) -> gtk::TextBuffer {
        let unit_info_text_view: &gtk::TextView = self.unit_info_textview.as_ref();

        let buf = unit_info_text_view.buffer();

        buf.set_text(""); // clear text
        buf
    }

    pub(super) fn register(&self, app_window: &AppWindow) {
        let activator = LinkActivator::new(Some(app_window.clone()));

        text_view_hyperlink::build_textview_link_platform(
            &self.unit_info_textview,
            self.hovering_over_link_tag.clone(),
            activator,
        );

        let text_search_bar_action_entry =
            text_search::create_action_entry(&self.text_search_bar, ACTION_FIND_IN_TEXT);

        app_window.add_action_entries([text_search_bar_action_entry]);

        if self.app_window.set(app_window.clone()).is_err() {
            warn!("Set only once");
        }
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

        self.update_unit_info(unit)
    }

    pub(super) fn set_inter_message(&self, action: &InterPanelMessage) {
        match *action {
            InterPanelMessage::FontProvider(old, new) => {
                set_text_view_font(old, new, &self.unit_info_textview);
                set_font_context(&self.unit_info_textview);
            }

            InterPanelMessage::UnitChange(unit) => self.set_unit(unit),
            InterPanelMessage::Refresh(unit) => self.refresh_panels(unit),
            _ => {}
        }
    }

    fn get_wrap(&self) -> bool {
        self.unit_info_textview.wrap_mode() != gtk::WrapMode::None
    }

    fn set_wrap(&self, wrap: bool) {
        let wrap_mode = if wrap {
            gtk::WrapMode::Word
        } else {
            gtk::WrapMode::None
        };
        self.unit_info_textview.set_wrap_mode(wrap_mode);
    }
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitInfoPanelImp {
    const NAME: &'static str = "UnitInfoPanel";
    type Type = super::UnitInfoPanel;
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
impl ObjectImpl for UnitInfoPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_sensitivity();

        set_font_context(&self.unit_info_textview);

        let settings = new_settings();

        settings
            .bind(KEY_PREF_UNIT_DESCRIPTION_WRAP, self.obj().as_ref(), "wrap")
            .build();

        text_search_construct(
            &self.unit_info_textview,
            &self.text_search_bar,
            &self.find_text_button,
            true,
        );

        settings
            .bind::<gtk::SearchBar>(
                &ACTION_FIND_IN_TEXT[4..],
                &self.text_search_bar,
                "search-mode-enabled",
            )
            .build();
    }
}

impl WidgetImpl for UnitInfoPanelImp {}
impl BoxImpl for UnitInfoPanelImp {}
