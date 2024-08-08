mod imp;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use log::{error, warn};

use crate::info::rowitem;
use crate::systemd;
use crate::systemd::data::UnitInfo;

glib::wrapper! {
    pub struct InfoWindow(ObjectSubclass<imp::InfoWindowImp>)
        @extends gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl InfoWindow {
    pub fn new() -> Self {
        // Create new window
        //let zelf = Object::builder().build();
        let obj: InfoWindow = glib::Object::new();

        let unit_prop_store = gio::ListStore::new::<rowitem::Metadata>();

        let no_selection = gtk::SingleSelection::new(Some(unit_prop_store.clone()));

        obj.imp().store.replace(Some(unit_prop_store));

        obj.imp()
            .unit_properties
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

                const SIZE: usize = 30;

                let mut tmp = String::new();
                let mut long_text = false;
                let key_label = if meta.col1().chars().count() > SIZE {
                    long_text = true;
                    tmp.push_str(&meta.col1()[..(SIZE - 3)]);
                    tmp.push_str("...");
                    &tmp
                } else {
                    tmp.push_str(meta.col1().as_str());
                    &tmp
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
                    .label(&meta.col2())
                    .selectable(true)
                    .build();

                box_.append(&l1);
                box_.append(&l2);

                box_.upcast::<gtk::Widget>()
            });

        obj
    }

    /*     fn setup_settings(&self) {
           let settings = gio::Settings::new(systemd_gui::APP_ID);
           self.imp()
               .settings
               .set(settings)
               .expect("`settings` should not be set before calling `setup_settings`.");
       }

       fn settings(&self) -> &gio::Settings {
           self.imp()
               .settings
               .get()
               .expect("`settings` should be set in `setup_settings`.")
       }

       pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
           // Get the size of the window
           let size = self.default_size();

           // Set the window state in `settings`
           self.settings().set_int("window-width", size.0)?;
           self.settings().set_int("window-height", size.1)?;
           self.settings()
               .set_boolean("is-maximized", self.is_maximized())?;

           Ok(())
       }

       fn load_window_size(&self) {
           // Get the window state from `settings`
           let mut width = self.settings().int("window-width");
           let mut height = self.settings().int("window-height");
           let is_maximized = self.settings().boolean("is-maximized");

           info!("Window settings: width {width}, height {height}, is-maximized {is_maximized}");

           if width < 0 {
               width = 1280;
           }

           if height < 0 {
               height = 720;
           }
           // Set the size of the window
           self.set_default_size(width, height);

           // If the window was maximized when it was closed, maximize it again
           if is_maximized {
               self.maximize();
           }
       }
    */
    fn load_dark_mode(&self) {}

    pub fn fill_data(&self, unit: &UnitInfo) {
        let unit_prop_store = &self.imp().store;

        if let Some(ref mut store) = *unit_prop_store.borrow_mut() {
            store.remove_all();

            match systemd::fetch_system_unit_info(&unit) {
                Ok(map) => {
                    for (key, value) in map {
                        store.append(&rowitem::Metadata::new(key, value));
                    }
                }
                Err(e) => error!("Fail to retreive Unit info: {:?}", e),
            }
        } else {
            warn!("Store not supposed to be None");
        };
    }
}
