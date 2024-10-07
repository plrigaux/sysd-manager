use std::cell::Ref;

use crate::{
    systemd::{analyze::*, SystemdErrors},
    widget::grid_cell::{Entry, GridCell},
};
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{object::Cast, BoxedAnyObject},
    pango::{AttrInt, AttrList, Weight},
    Orientation, Window,
};

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

    unit_analyse_box.append({
        let attribute_list = AttrList::new();
        attribute_list.insert(AttrInt::new_weight(Weight::Medium));
        &gtk::Label::builder()
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
    let analyze_tree = setup_systemd_analyze_tree(&total_time_label)?;

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&analyze_tree)
        .build();

    unit_analyse_box.append(&total_time_label);
    unit_analyse_box.append(&unit_analyse_scrolled_window);

    Ok(unit_analyse_box)
}

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze_tree(
    total_time_label: &gtk::Label,
) -> Result<gtk::ColumnView, SystemdErrors> {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let units = blame()?;

    let mut time_full = 0;

    for value in units.into_iter() {
        time_full = value.time;
        store.append(&BoxedAnyObject::new(value));
    }

    let single_selection = gtk::SingleSelection::new(Some(store));
    /*     let analyze_tree = gtk::ColumnView::new(Some(single_selection));
    analyze_tree.set_focusable(true); */
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

    let time = (time_full as f32) / 1000f32;
    total_time_label.set_label(format!("{} seconds", time).as_str());

    Ok(analyze_tree)
}
