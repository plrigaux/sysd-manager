use std::{
    cell::{Cell, RefCell},
    rc::Rc,
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

use log::{info, warn};

use crate::{
    systemd::data::UnitInfo,
    utils::{
        font_management::{set_font_context, set_text_view_font},
        text_view_hyperlink::{self, LinkActivator},
        writer::UnitInfoWriter,
    },
    widget::{InterPanelMessage, app_window::AppWindow, info_window::InfoWindow},
};

use super::construct_info::fill_all_info;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info_panel.ui")]
pub struct UnitInfoPanelImp {
    #[template_child]
    show_all_button: TemplateChild<gtk::Button>,

    #[template_child]
    refresh_button: TemplateChild<gtk::Button>,

    #[template_child]
    unit_info_textview: TemplateChild<gtk::TextView>,

    unit: RefCell<Option<UnitInfo>>,

    is_dark: Cell<bool>,

    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,
}

#[gtk::template_callbacks]
impl UnitInfoPanelImp {
    #[template_callback]
    fn refresh_info_clicked(&self, button: &gtk::Button) {
        info!("button {button:?}");

        self.refresh_panels();
    }

    #[template_callback]
    fn show_all_clicked(&self, _button: &gtk::Button) {
        let binding = self.unit.borrow();
        let Some(unit) = binding.as_ref() else {
            warn!("no unit file");
            return;
        };

        let info_window = InfoWindow::new(Some(unit));

        info!("show_all_clicked {:?}", unit.primary());

        info_window.present();
    }
}

impl UnitInfoPanelImp {
    pub(crate) fn set_unit(&self, unit: Option<&UnitInfo>) {
        match unit {
            Some(unit) => {
                let _old = self.unit.replace(Some(unit.clone()));

                self.update_unit_info(unit)
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

        let is_dark = self.is_dark.get();

        let mut info_writer = UnitInfoWriter::new(buf, start_iter, is_dark);

        fill_all_info(unit, &mut info_writer);
    }

    fn clear(&self) -> gtk::TextBuffer {
        let unit_info_text_view: &gtk::TextView = self.unit_info_textview.as_ref();

        let buf = unit_info_text_view.buffer();

        buf.set_text(""); // clear text
        buf
    }

    pub(crate) fn set_dark(&self, is_dark: bool) {
        self.is_dark.set(is_dark);
    }

    pub(crate) fn register(&self, app_window: &AppWindow) {
        {
            let app_window = app_window.clone();

            let activator = LinkActivator::new(Some(app_window));

            text_view_hyperlink::build_textview_link_platform(
                &self.unit_info_textview,
                self.hovering_over_link_tag.clone(),
                activator,
            );
        }
    }

    pub(super) fn refresh_panels(&self) {
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
            InterPanelMessage::IsDark(is_dark) => self.set_dark(is_dark),

            InterPanelMessage::UnitChange(unit) => self.set_unit(unit),
            _ => {}
        }
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

impl ObjectImpl for UnitInfoPanelImp {
    fn constructed(&self) {
        self.parent_constructed();

        self.set_sensitivity();

        set_font_context(&self.unit_info_textview);
    }
}
impl WidgetImpl for UnitInfoPanelImp {}
impl BoxImpl for UnitInfoPanelImp {}
