use std::cell::Ref;

use crate::{
    systemd::{
        analyze::{self, Analyze},
        errors::SystemdErrors,
    },
    widget::{
        grid_cell::{Entry, GridCell},
        unit_file_panel::flatpak,
    },
};

use gtk::{
    Orientation, TextView, Window,
    gio::{self},
    glib::{self, BoxedAnyObject, object::Cast},
    pango::{AttrInt, AttrList, Weight},
    prelude::*,
};
use log::{info, warn};

const PAGE_BLAME: &str = "blame";

pub fn build_analyze_window() -> Result<Window, SystemdErrors> {
    let analyse_box = build_analyze()?;

    let window = Window::builder()
        .title("Analyse Blame")
        .default_height(600)
        .default_width(600)
        .child(&analyse_box)
        .build();

    Ok(window)
}

fn build_analyze() -> Result<gtk::Box, SystemdErrors> {
    // Analyse
    let unit_analyse_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    unit_analyse_box.append(&{
        let attribute_list = AttrList::new();
        attribute_list.insert(AttrInt::new_weight(Weight::Medium));
        gtk::Label::builder()
            .label("Total Time:")
            .attributes(&attribute_list)
            .build()
    });

    let attribute_list = AttrList::new();
    attribute_list.insert(AttrInt::new_weight(Weight::Medium));
    let total_time_label = gtk::Label::builder()
        .label("seconds ...")
        .attributes(&attribute_list)
        .build();

    // Setup the Analyze stack
    let (analyze_tree, store) = setup_systemd_analyze_tree()?;

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&analyze_tree)
        .build();

    let stack = gtk::Stack::new();

    let spinner = adw::Spinner::new();

    stack.add_named(&spinner, Some("spinner"));
    stack.add_named(&unit_analyse_scrolled_window, Some(PAGE_BLAME));

    unit_analyse_box.append(&total_time_label);
    unit_analyse_box.append(&stack);

    fill_store(&store, &total_time_label, &stack);

    Ok(unit_analyse_box)
}

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze_tree() -> Result<(gtk::ColumnView, gio::ListStore), SystemdErrors> {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let single_selection = gtk::SingleSelection::new(Some(store.clone()));

    let analyze_tree = gtk::ColumnView::builder()
        .focusable(true)
        .model(&single_selection)
        .hexpand(true)
        .build();

    let col1factory = gtk::SignalListItemFactory::new();
    let col2factory = gtk::SignalListItemFactory::new();

    col1factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = GridCell::default();
        item.set_child(Some(&row));
    });

    col1factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<GridCell>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<Analyze> = entry.borrow();
        let ent = Entry {
            name: r.time.to_string(),
        };
        child.set_entry(&ent);
    });

    col2factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = GridCell::default();
        item.set_child(Some(&row));
    });

    col2factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<GridCell>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<Analyze> = entry.borrow();
        let ent = Entry {
            name: r.service.to_string(),
        };
        child.set_entry(&ent);
    });

    let col1_time = gtk::ColumnViewColumn::new(Some("Init time (ms)"), Some(col1factory));
    let col2_unit = gtk::ColumnViewColumn::new(Some("Running units"), Some(col2factory));
    col2_unit.set_expand(true);

    analyze_tree.append_column(&col1_time);
    analyze_tree.append_column(&col2_unit);

    Ok((analyze_tree, store))
}

fn fill_store(list_store: &gio::ListStore, total_time_label: &gtk::Label, stack: &gtk::Stack) {
    {
        let list_store = list_store.clone();
        let total_time_label = total_time_label.clone();
        let stack = stack.clone();

        glib::spawn_future_local(async move {
            let units_rep = gio::spawn_blocking(move || match analyze::blame() {
                Ok(units) => Ok(units),
                Err(error) => {
                    warn!("Analyse blame Error {:?}", error);
                    Err(error)
                }
            })
            .await
            .expect("Task needs to finish successfully.");

            list_store.remove_all();
            let mut time_full = 0;

            match units_rep {
                Ok(units) => {
                    for value in units {
                        time_full = value.time;
                        list_store.append(&BoxedAnyObject::new(value));
                    }

                    info!("Unit list refreshed! list size {}", list_store.n_items());

                    let time = (time_full as f32) / 1000f32;
                    total_time_label.set_label(format!("{} seconds", time).as_str());
                    stack.set_visible_child_name(PAGE_BLAME);
                }
                Err(error) => {
                    const FLATPACK_PERMISSION: &str = "flatpak_permission";

                    match error {
                        SystemdErrors::CmdNoFreedesktopFlatpakPermission(cmd, _) => {
                            let dialog = flatpak::inner_msg(cmd, None);

                            stack.add_named(&dialog, Some(FLATPACK_PERMISSION));
                            stack.set_visible_child_name(FLATPACK_PERMISSION)
                        }
                        SystemdErrors::CmdNoFlatpakSpawn => {
                            let tv = TextView::new();
                            let buf = tv.buffer();

                            let mut start_iter = buf.start_iter();
                            let gui_description = error.gui_description().unwrap_or(String::new());
                            buf.insert_markup(&mut start_iter, &gui_description);

                            stack.add_named(&tv, Some(FLATPACK_PERMISSION));
                            stack.set_visible_child_name(FLATPACK_PERMISSION)
                        }
                        _ => stack.set_visible_child_name(PAGE_BLAME),
                    };
                }
            }
        });
    }
}
