use std::cell::Ref;

use gtk::glib::BoxedAnyObject;
use gtk::{gio, prelude::ActionMapExtManual};
use gtk::{prelude::*, ListBox};

use crate::analyze::build_analyze_window;
use log::error;
use systemd;

fn build_popover_menu() -> gtk::PopoverMenu {
    let menu = gio::Menu::new();

    menu.append(Some("Analyze Blame"), Some("app.analyze_blame"));
    menu.append(Some("About"), Some("app.about"));
    menu.append(Some("Systemd Info"), Some("app.systemd_info"));

    let unit_menu_popover = gtk::PopoverMenu::builder().menu_model(&menu).build();

    unit_menu_popover
}

pub fn build_menu() -> gtk::MenuButton {
    let popover = build_popover_menu();
    let menu_button = gtk::MenuButton::builder()
        .focusable(true)
        .receives_default(true)
        .icon_name("open-menu-symbolic")
        .halign(gtk::Align::End)
        .direction(gtk::ArrowType::Down)
        .popover(&popover)
        .build();

    menu_button
}

pub fn on_startup(app: &gtk::Application) {
    let about = gio::ActionEntry::builder("about")
        .activate(|_, _, _| {
            let about = create_about();
            about.present();
        })
        .build();

    let analyze_blame = gio::ActionEntry::builder("analyze_blame")
        .activate(|_, _, _| {
            let analyze_blame_window = build_analyze_window();
            analyze_blame_window.present();
        })
        .build();

    let systemd_info = gio::ActionEntry::builder("systemd_info")
        .activate(|_, _, _| {
            let analyze_blame_window = build_systemd_info();
            analyze_blame_window.present();
        })
        .build();

    app.add_action_entries([about, analyze_blame, systemd_info]);
}

fn create_about() -> gtk::AboutDialog {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let about = gtk::AboutDialog::builder()
        .authors(["Pierre-Luc Rigaux"])
        .name("About")
        .program_name("SysD manager")
        .modal(true)
        .version(VERSION)
        .comments("This is comments")
        .build();

    about
}

fn build_systemd_info() -> gtk::Window {
    let systemd_info = build_systemd_info_data();

    let window = gtk::Window::builder()
        .title("Systemd Info Blame")
        .default_height(600)
        .default_width(600)
        .child(&systemd_info)
        .build();

    window
}

fn build_systemd_info_data() -> gtk::ScrolledWindow {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let Ok(map) = systemd::fetch_system_info() else {
        let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .focusable(true)
            .build();

        return unit_analyse_scrolled_window;
    };

    struct RowItem {
        key: String,
        value: String,
    }
    for (key, value) in map {
        store.append(&BoxedAnyObject::new(RowItem { key, value }));
    }

    let no_selection = gtk::SingleSelection::new(Some(store));

    let list_box = ListBox::builder().build();

    list_box.bind_model(Some(&no_selection), move |object| {
        let cloned_object = <gtk::glib::Object as Clone>::clone(&object);
       
        let box_any = match cloned_object.downcast::<BoxedAnyObject>() {
            Ok(any_objet) => any_objet,
            Err(val) => {
                error!("Bind  Error: {:?}", val);
                let list_box_row = gtk::ListBoxRow::new();
                return list_box_row.upcast::<gtk::Widget>();
            }
        };

        let unit: Ref<RowItem> = box_any.borrow();

        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 15);

        let l1 = gtk::Label::new(Some(&unit.key));
        l1.set_width_chars(30);
        l1.set_halign(gtk::Align::End);
        //l1.set_wrap(true);
        let l2 = gtk::Label::new(Some(&unit.value));

        box_.append(&l1);
        box_.append(&l2);

        box_.upcast::<gtk::Widget>()
    });
    //list_box.set_mo

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&list_box)
        .build();

    unit_analyse_scrolled_window
}
/*
//https://github.com/gtk-rs/gtk4-rs/blob/master/examples/column_view_datagrid/main.rs

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze_tree(total_time_label: &gtk::Label) -> gtk::ColumnView {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let units = Analyze::blame();

    for value in units.clone() {
        //debug!("Analyse Tree Blame {:?}", value);
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

    let time = (units.iter().last().unwrap().time as f32) / 1000f32;
    total_time_label.set_label(format!("{} seconds", time).as_str());

    analyze_tree
}
 */
