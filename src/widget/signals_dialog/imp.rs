use std::cell::OnceCell;

use adw::subclass::window::AdwWindowImpl;
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
use log::info;

use crate::widget::app_window::AppWindow;

use super::SignalsWindow;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/signals_window.ui")]
pub struct SignalsWindowImp {
    #[template_child]
    list_box: TemplateChild<gtk::ListBox>,

    app_window: OnceCell<AppWindow>,
}

#[gtk::template_callbacks]
impl SignalsWindowImp {
    pub(crate) fn set_app_window(&self, app_window: Option<&AppWindow>) {
        if let Some(app_window) = app_window {
            self.app_window
                .set(app_window.clone())
                .expect("app_window set once");
        }
    }
}
// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SignalsWindowImp {
    const NAME: &'static str = "SIGNALS_DIALOG";
    type Type = SignalsWindow;
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

impl ObjectImpl for SignalsWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for SignalsWindowImp {}
impl WindowImpl for SignalsWindowImp {
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        info!("Close window");

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for SignalsWindowImp {}
