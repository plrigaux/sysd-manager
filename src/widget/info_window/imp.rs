use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;

// ANCHOR: imp
#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info.ui")]
pub struct InfoWindowImp {
    //pub settings: OnceCell<Settings>,
    #[template_child]
    pub unit_properties: TemplateChild<gtk::ListBox>,

    pub(super) store: RefCell<Option<gio::ListStore>>,
}

#[glib::object_subclass]
impl ObjectSubclass for InfoWindowImp {
    const NAME: &'static str = "InfoWindow";
    type Type = super::InfoWindow;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for InfoWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
        // Load latest window state
        let obj = self.obj();
        // obj.setup_settings();
        // obj.load_window_size();
        obj.load_dark_mode();
    }
}
impl WidgetImpl for InfoWindowImp {}
impl WindowImpl for InfoWindowImp {
    // Save window state right before the window will be closed
    fn close_request(&self) -> glib::Propagation {
        // Save window size
        log::debug!("Close window");
        /*         self.obj()
        .save_window_size()
        .expect("Failed to save window state"); */
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl ApplicationWindowImpl for InfoWindowImp {}
// ANCHOR_END: imp
