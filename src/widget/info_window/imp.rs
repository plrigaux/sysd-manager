use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use log::{debug, error, warn};
use std::cell::{OnceCell, RefCell};

use crate::systemd;
use crate::systemd::data::UnitInfo;
use crate::systemd_gui::new_settings;

use super::rowitem;

const WINDOW_WIDTH: &str = "info-window-width";
const WINDOW_HEIGHT: &str = "info-window-height";
const IS_MAXIMIZED: &str = "info-is-maximized";

// ANCHOR: imp
#[derive(Debug, Default, gtk::CompositeTemplate)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_info.ui")]
pub struct InfoWindowImp {
    #[template_child]
    pub unit_properties: TemplateChild<gtk::ListBox>,

    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    pub(super) store: RefCell<Option<gio::ListStore>>,

    last_filter_string: RefCell<String>,

    custom_filter: OnceCell<gtk::CustomFilter>,

    settings: OnceCell<gio::Settings>,
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

    #[template_callback]
    fn search_changed(&self, search_entry: &gtk::SearchEntry) {
        let text = search_entry.text();

        debug!("Search text \"{text}\"");

        let mut last_filter = self.last_filter_string.borrow_mut();

        let change_type = if text.is_empty() {
            gtk::FilterChange::LessStrict
        } else if text.len() > last_filter.len() && text.contains(last_filter.as_str()) {
            gtk::FilterChange::MoreStrict
        } else if text.len() < last_filter.len() && last_filter.contains(text.as_str()) {
            gtk::FilterChange::LessStrict
        } else {
            gtk::FilterChange::Different
        };

        debug!("Current \"{}\" Prev \"{}\"", text, last_filter);
        last_filter.replace_range(.., text.as_str());

        if let Some(custom_filter) = self.custom_filter.get() {
            custom_filter.changed(change_type);
        }
    }
}

impl InfoWindowImp {
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

    fn create_filter(&self) -> gtk::CustomFilter {
        let search_entry = self.search_entry.clone();

        gtk::CustomFilter::new(move |object| {
            let Some(meta) = object.downcast_ref::<rowitem::Metadata>() else {
                error!("some wrong downcast_ref {:?}", object);
                return false;
            };

            let text = search_entry.text();

            if text.is_empty() {
                return true;
            }

            let texts = text.as_str();
            if text.chars().any(|c| c.is_ascii_uppercase()) {
                meta.col1().contains(texts) || meta.col2().contains(texts)
            } else {
                meta.col1().to_ascii_lowercase().contains(texts)
                    || meta.col2().to_ascii_lowercase().contains(texts)
            }
        })
    }

    fn settings(&self) -> &gio::Settings {
        match self.settings.get() {
            Some(settings) => settings,
            None => {
                let settings: gio::Settings = new_settings();

                self.settings
                    .set(settings)
                    .expect("`settings` should not be set before calling `setup_settings`.");

                self.settings.get().expect("`settings` should be set ")
            }
        }
        //.expect("`settings` should be set in `setup_settings`.")
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let settings = self.settings();

        let mut width = settings.int(WINDOW_WIDTH);
        let mut height = settings.int(WINDOW_HEIGHT);
        let is_maximized = settings.boolean(IS_MAXIMIZED);

        let obj = self.obj();
        let (def_width, def_height) = obj.default_size();

        if width <= 0 {
            width = def_width;
            if width <= 0 {
                width = 650;
            }
        }

        if height <= 0 {
            height = def_height;
            if height <= 0 {
                height = 600;
            }
        }

        // Set the size of the window
        obj.set_default_size(width, height);

        // If the window was maximized when it was closed, maximize it again
        if is_maximized {
            obj.maximize();
        }
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        // Get the size of the window

        let obj = self.obj();
        let (width, height) = obj.default_size();

        // Set the window state in `settings`
        let settings = self.settings();

        settings.set_int(WINDOW_WIDTH, width)?;
        settings.set_int(WINDOW_HEIGHT, height)?;
        settings.set_boolean(IS_MAXIMIZED, obj.is_maximized())?;

        Ok(())
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

        self.load_window_size();
        let unit_prop_store = gio::ListStore::new::<rowitem::Metadata>();

        let no_selection = gtk::NoSelection::new(Some(unit_prop_store.clone()));

        let filter = self.create_filter();
        let _ = self.custom_filter.set(filter.clone());
        let filtering_model = gtk::FilterListModel::new(Some(no_selection), Some(filter));

        self.store.replace(Some(unit_prop_store));

        self.unit_properties
            .bind_model(Some(&filtering_model), |object| {
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
        if let Err(_err) = self.save_window_size() {
            error!("Failed to save window state");
        }

        self.parent_close_request();
        // Allow to invoke other event handlers
        glib::Propagation::Proceed
    }
}
impl ApplicationWindowImpl for InfoWindowImp {}
// ANCHOR_END: imp
