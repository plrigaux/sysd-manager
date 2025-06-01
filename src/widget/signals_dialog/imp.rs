use std::cell::{OnceCell, Ref, RefCell};

use adw::subclass::window::AdwWindowImpl;
use gio::glib::BoxedAnyObject;
use gtk::{
    glib::{self},
    prelude::*,
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use log::{debug, info};
use tokio::sync::mpsc;

use crate::{
    systemd::{SystemdSignal, runtime, watch_systemd_signals},
    systemd_gui::new_settings,
    widget::app_window::AppWindow,
};

use super::{SignalsWindow, signal_row::SignalRow};

const SIGNAL_WINDOW_WIDTH: &str = "signal-window-width";
const SIGNAL_WINDOW_HEIGHT: &str = "signal-window-height";

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/signals_window.ui")]
pub struct SignalsWindowImp {
    #[template_child]
    signals_list: TemplateChild<gtk::ListView>,

    #[template_child]
    panel_stack: TemplateChild<adw::ViewStack>,

    signals: RefCell<Option<gio::ListStore>>,

    app_window: OnceCell<AppWindow>,

    token: OnceCell<tokio_util::sync::CancellationToken>,
}

#[gtk::template_callbacks]
impl SignalsWindowImp {
    pub(crate) fn set_app_window(&self, app_window: &AppWindow) {
        self.app_window
            .set(app_window.clone())
            .expect("app_window set once");
    }

    fn setup_factory(&self) -> gtk::SignalListItemFactory {
        let factory = gtk::SignalListItemFactory::new();

        // Create an empty `TaskRow` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `TaskRow`
            let signal_row = SignalRow::new();

            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&signal_row));
        });

        // Tell factory how to bind `TaskRow` to a `TaskObject`
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");

            let task_object = list_item
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("The item has to be an `TaskObject`.");

            // Get `TaskRow` from `ListItem`
            let signal_row = list_item
                .child()
                .and_downcast::<SignalRow>()
                .expect("The child has to be a `SignalRow`.");

            let r: Ref<SystemdSignal> = task_object.borrow();
            signal_row.set_type_text(r.type_text());

            signal_row.set_details_text(&r.details());
        });

        // Tell factory how to unbind `TaskRow` from `TaskObject`
        factory.connect_unbind(move |_, _list_item| {});

        factory
    }

    fn display_signals(&self) {
        self.panel_stack.set_visible_child_name("signals");
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
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        self.signals.replace(Some(model.clone()));
        let selection_model = gtk::NoSelection::new(Some(model.clone()));
        self.signals_list.set_model(Some(&selection_model));

        let factory = self.setup_factory();
        self.signals_list.set_factory(Some(&factory));

        let signal_dialog = self.obj().clone();
        let (systemd_signal_sender, mut systemd_signal_receiver) = mpsc::channel(100);

        glib::spawn_future_local(async move {
            if let Some(signal) = systemd_signal_receiver.recv().await {
                debug!("Recieve {:?}", signal);
                let boxed = BoxedAnyObject::new(signal);
                model.append(&boxed);

                signal_dialog.imp().display_signals();
            }

            while let Some(signal) = systemd_signal_receiver.recv().await {
                debug!("Recieve {:?}", signal);
                let boxed = BoxedAnyObject::new(signal);
                model.append(&boxed);
            }
        });

        let cancellation_token = tokio_util::sync::CancellationToken::new();

        let _ = self.token.set(cancellation_token.clone());

        runtime().spawn(watch_systemd_signals(
            systemd_signal_sender,
            cancellation_token,
        ));

        let settings = new_settings();

        let width = settings.int(SIGNAL_WINDOW_WIDTH);
        let height = settings.int(SIGNAL_WINDOW_HEIGHT);

        self.obj().set_default_size(width, height);
    }
}

impl WidgetImpl for SignalsWindowImp {}
impl WindowImpl for SignalsWindowImp {
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        info!("Close window signals");

        if let Some(token) = self.token.get() {
            token.cancel();
        }

        self.app_window
            .get()
            .expect("Window not None")
            .set_signal_window(None);

        let (width, height) = self.obj().default_size();

        let settings = new_settings();

        let _ = settings.set_int(SIGNAL_WINDOW_WIDTH, width);
        let _ = settings.set_int(SIGNAL_WINDOW_HEIGHT, height);

        self.parent_close_request();

        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl AdwWindowImpl for SignalsWindowImp {}
