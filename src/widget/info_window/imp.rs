use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use log::{error, warn};
use std::cell::RefCell;

use crate::systemd;
use crate::systemd::data::UnitInfo;

use super::rowitem;

// ANCHOR: imp
#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info.ui")]
pub struct InfoWindowImp {
    //pub settings: OnceCell<Settings>,
    #[template_child]
    pub unit_properties: TemplateChild<gtk::ListBox>,

    pub(super) store: RefCell<Option<gio::ListStore>>,
}

#[gtk::template_callbacks]
impl InfoWindowImp {
    #[template_callback]
    fn handle_copy_click(&self, _button: &gtk::Button) {
        let clipboard = _button.clipboard();

        let unit_prop_store = &self.store;
        //unit_prop_store.borrow()
        if let Some(store) = unit_prop_store.borrow().as_ref() {
            let n_item = store.n_items();

            let mut data = String::new();
            for i in 0..n_item {
                if let Some(object) = store.item(i) {
                    if let Ok(x) = object.downcast::<rowitem::Metadata>() {
                        data.push_str(&x.col1());
                        data.push('\t');
                        data.push_str(&x.col2());
                        data.push('\n')
                    }
                }
            }
            clipboard.set_text(&data)
        }
    }

    pub fn fill_data(&self, unit: &UnitInfo) {
        let unit_prop_store = &self.store;

        if let Some(ref mut store) = *unit_prop_store.borrow_mut() {
            store.remove_all();

            match systemd::fetch_system_unit_info(unit) {
                Ok(map) => {
                    for (idx, (key, value)) in map.into_iter().enumerate() {
                        //println!("{key} :-: {value}");
                        let data = rowitem::Metadata::new(idx as u32, key, value);
                        store.append(&data);
                    }
                }
                Err(e) => warn!("Fails to retreive Unit info: {:?}", e),
            }
        } else {
            warn!("Store not supposed to be None");
        };

        let mut title = String::from("Unit Info - ");
        title.push_str(&unit.primary());
        self.obj().set_title(Some(&title));
    }

    pub fn fill_systemd_info(&self) {
        let unit_prop_store = &self.store;

        if let Some(ref mut store) = *unit_prop_store.borrow_mut() {
            store.remove_all();

            match systemd::fetch_system_info() {
                Ok(map) => {
                    for (idx, (key, value)) in map.into_iter().enumerate() {
                        //println!("{key} :-: {value}");
                        let data = rowitem::Metadata::new(idx as u32, key, value);
                        store.append(&data);
                    }
                }
                Err(e) => error!("Fail to retreive Unit info: {:?}", e),
            }
        } else {
            warn!("Store not supposed to be None");
        };

        self.obj().set_title(Some("Systemd Info"));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for InfoWindowImp {
    const NAME: &'static str = "InfoWindow";
    type Type = super::InfoWindow;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for InfoWindowImp {
    fn constructed(&self) {
        self.parent_constructed();
        // Load latest window state

        let unit_prop_store = gio::ListStore::new::<rowitem::Metadata>();

        let no_selection = gtk::SingleSelection::new(Some(unit_prop_store.clone()));

        self.store.replace(Some(unit_prop_store));

        self.unit_properties
            .bind_model(Some(&no_selection), |object| {
                let meta = match object.downcast_ref::<rowitem::Metadata>() {
                    Some(any_objet) => any_objet,
                    None => {
                        error!("No linked object");
                        let list_box_row = gtk::ListBoxRow::new();
                        return list_box_row.upcast::<gtk::Widget>();
                    }
                };

                let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 15);

                const SIZE: usize = 36;

                let mut long_text = false;
                let col1 = meta.col1();
                let key_label = if col1.chars().count() > SIZE {
                    long_text = true;
                    let mut tmp = String::new();
                    tmp.push_str(&col1[..(SIZE - 3)]);
                    tmp.push_str("...");
                    tmp
                } else {
                    col1
                };

                let l1 = gtk::Label::builder()
                    .label(key_label)
                    .width_chars(SIZE as i32)
                    .xalign(0.0)
                    .max_width_chars(30)
                    .single_line_mode(true)
                    .selectable(true)
                    .build();

                if long_text {
                    l1.set_tooltip_text(Some(&meta.col1()));
                }

                let l2 = gtk::Label::builder()
                    .label(meta.col2())
                    .selectable(true)
                    .build();

                let idx = meta.col0().to_string();
                let l0 = gtk::Label::builder()
                    .label(idx)
                    .width_chars(3)
                    .selectable(false)
                    .css_classes(["idx"])
                    .build();

                box_.append(&l0);
                box_.append(&l1);
                box_.append(&l2);

                box_.upcast::<gtk::Widget>()
            });
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
