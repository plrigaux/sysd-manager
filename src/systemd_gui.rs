use gtk::{self, SingleSelection};

use gtk::prelude::*;
use systemd::analyze::Analyze;

use crate::grid_cell::{Entry, GridCell};
use crate::systemd::get_unit_journal;

use systemd::{self, EnablementStatus, LoadedUnit};

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};

use gtk::{Application, ApplicationWindow, Orientation};

use gtk::gio;
use std::cell::Ref;

// ANCHOR: main
const APP_ID: &str = "org.systemd.manager";

const _ICON_YES: &str = "object-select-symbolic";
const _ICON_NO: &str = "window-close-symbolic";

/// Updates the status icon for the selected unit
/* fn update_icon(icon: &gtk::Image, state: bool) {
    if state {
        icon.set_from_icon_name(Some(ICON_YES));
    } else {
        icon.set_from_icon_name(Some(ICON_NO));
    }
}
 */
/// Create a `gtk::ListboxRow` and add it to the `gtk::ListBox`, and then add the `gtk::Image` to a vector so that we can later modify
/// it when the state changes.
/* fn create_row(systemd_unit: &LoadedUnit, state_icons: &mut Vec<gtk::Image>) -> gtk::ListBoxRow {
    let unit_box = gtk::CenterBox::new();
    let unit_label = gtk::Label::new(Some(&systemd_unit.display_name()));
    let image = if systemd_unit.is_enable() {
        gtk::Image::from_icon_name(ICON_YES)
    } else {
        gtk::Image::from_icon_name(ICON_NO)
    };

    unit_box.set_start_widget(Some(&unit_label));
    unit_box.set_end_widget(Some(&image));

    let unit_row = gtk::ListBoxRow::builder().child(&unit_box).build();

    state_icons.push(image);

    unit_row
} */

#[macro_export]
macro_rules! get_selected_unit {
    ( $column_view:expr  ) => {{
        let Some(model) = $column_view.model() else {
            panic!("Can't find model")
        };

        let Some(single_selection_model) = model.downcast_ref::<SingleSelection>() else {
            panic!("Can't downcast to SingleSelection")
        };

        let Some(object) = single_selection_model.selected_item() else {
            eprintln!("No selection objet");
            return;
        };

        let box_any = match object.downcast::<BoxedAnyObject>() {
            Ok(any_objet) => any_objet,
            Err(val) => {
                eprintln!("Selection Error: {:?}", val);
                return;
            }
        };
        box_any
    }};
}

//https://github.com/gtk-rs/gtk4-rs/blob/master/examples/column_view_datagrid/main.rs

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze_tree(total_time_label: &gtk::Label) -> gtk::ColumnView {
    let store = gio::ListStore::new::<BoxedAnyObject>();

    let units = Analyze::blame();

    for value in units.clone() {
        //println!("Analyse Tree Blame {:?}", value);
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

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &LoadedUnit) {
    let journal_output = get_unit_journal(unit);
    journal.buffer().set_text(&journal_output);
}

pub fn launch() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

const SERVICES_TITLE: &str = "Services";
const SOCKETS_TITLE: &str = "Sockets";
const TIMERS_TITLE: &str = "Timers";

fn build_popover_menu(
    _menu_button: &gtk::MenuButton,
    /*   _unit_stack: &gtk::Stack, */
) -> gtk::PopoverMenu {
    let services_button = gtk::Button::builder()
        .label(SERVICES_TITLE)
        .focusable(true)
        .receives_default(true)
        .build();

    let sockets_button = gtk::Button::builder()
        .label(SOCKETS_TITLE)
        .focusable(true)
        .receives_default(true)
        .build();

    let timers_button = gtk::Button::builder()
        .label(TIMERS_TITLE)
        .focusable(true)
        .receives_default(true)
        .build();

    let unit_menu_popover = gtk::PopoverMenu::builder()
        .child(&{
            let g_box = gtk::Box::new(Orientation::Vertical, 0);
            g_box.append(&services_button);
            g_box.append(&sockets_button);
            g_box.append(&timers_button);
            g_box
        })
        .build();

    // let popover = RefCell::new(unit_menu_popover);
    {
        /*         let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone(); */
        services_button.connect_clicked(move |_| {
            /*             stack.set_visible_child_name(SERVICES_TITLE);
            mb.set_label(SERVICES_TITLE);
            popover.set_visible(false); */
        });
    }

    {
        /*         let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone(); */
        sockets_button.connect_clicked(move |_| {
            /*             stack.set_visible_child_name(SOCKETS_TITLE);
            mb.set_label(SOCKETS_TITLE);
            popover.set_visible(false); */
        });
    }

    {
        /*         let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone(); */
        timers_button.connect_clicked(move |_| {
            /*             stack.set_visible_child_name(TIMERS_TITLE);
            mb.set_label(TIMERS_TITLE);
            popover.set_visible(false); */
        });
    }

    unit_menu_popover
}

fn build_ui(application: &Application) {
    // List of all unit files on the system
    let unit_files: Vec<LoadedUnit> = match systemd::list_units_description_and_state() {
        Ok(map) => map.into_values().collect(),
        Err(e) => {
            println!("{:?}", e);
            vec![]
        }
    };

    let store = gtk::gio::ListStore::new::<BoxedAnyObject>();

        for value in unit_files.clone() {
        //println!("Analyse Tree Blame {:?}", value);
        store.append(&BoxedAnyObject::new(value));
    }

    let columnview_selection_model = gtk::SingleSelection::new(Some(store));

    /*     let column_view = gtk::ColumnView::new(Some(selection_model));
    column_view.set_focusable(true); */

    let column_view = gtk::ColumnView::builder()
        .model(&columnview_selection_model)
        .focusable(true)
        .build();

    let col1factory = gtk::SignalListItemFactory::new();
    let col2factory = gtk::SignalListItemFactory::new();
    let col3factory = gtk::SignalListItemFactory::new();

    col1factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col1factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<LoadedUnit> = entry.borrow();
        child.set_label(r.display_name());
    });

    col2factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col2factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<LoadedUnit> = entry.borrow();
        child.set_label(r.enable_status());
    });

    col3factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });
    col3factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<LoadedUnit> = entry.borrow();
        child.set_label(r.unit_type());
    });

    let col1_unit = gtk::ColumnViewColumn::new(Some("Unit"), Some(col1factory));
    col1_unit.set_resizable(true);
    col1_unit.set_fixed_width(140);

    let col3_unit = gtk::ColumnViewColumn::new(Some("Type"), Some(col3factory));
    col3_unit.set_resizable(true);
    col3_unit.set_fixed_width(75);

    let col2_enable_status = gtk::ColumnViewColumn::new(Some("Enable status"), Some(col2factory));
    col2_enable_status.set_expand(true);

    column_view.append_column(&col1_unit);
    column_view.append_column(&col3_unit);
    column_view.append_column(&col2_enable_status);

    let unit_col_view_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&column_view)
        .build();

    let left_pane = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .width_request(350)
        .build();

    left_pane.append(&unit_col_view_scrolled_window);
    //-------------------------------------------

    let unit_info = gtk::TextView::builder()
        .focusable(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(5)
        .right_margin(5)
        .monospace(true)
        .build();

    let unit_file_stack_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&unit_info)
        .build();

    let save_unit_file_button = gtk::Button::builder()
        .label("gtk-save")
        .focusable(true)
        .receives_default(true)
        .build();

    let unit_file_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    unit_file_box.append(&unit_file_stack_scrolled_window);
    unit_file_box.append(&save_unit_file_button);

    let unit_journal_view = gtk::TextView::builder()
        .focusable(true)
        .editable(false)
        .accepts_tab(false)
        .build();

    let unit_journal_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&unit_journal_view)
        .build();

    let unit_journal_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    let refresh_log_button = gtk::Button::builder()
        .label("Refresh")
        .focusable(true)
        .receives_default(true)
        .build();

    unit_journal_box.append(&unit_journal_scrolled_window);
    unit_journal_box.append(&refresh_log_button);
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
    let analyze_tree = setup_systemd_analyze_tree(&total_time_label);

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&analyze_tree)
        .build();

    unit_analyse_box.append(&total_time_label);
    unit_analyse_box.append(&unit_analyse_scrolled_window);

    let info_stack = gtk::Stack::builder()
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    info_stack.add_titled(&unit_file_box, Some("Unit File"), "Unit File");
    info_stack.add_titled(&unit_journal_box, Some("Unit Journal"), "Unit Journal");
    info_stack.add_titled(&unit_analyse_box, Some("Analyze"), "Analyze");

    let stack_switcher = gtk::StackSwitcher::builder().stack(&info_stack).build();

    let right_pane = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .hexpand(true)
        .build();

    right_pane.append(&stack_switcher);
    right_pane.append(&info_stack);

    // ---------------------------------------------------

    let main_box = gtk::Box::new(Orientation::Horizontal, 5);
    main_box.append(&left_pane);
    main_box.append(&right_pane);

    // ----------------------------------------------

    let menu_button = gtk::MenuButton::builder()
        .focusable(true)
        .receives_default(true)
        .label(SERVICES_TITLE)
        .build();

    menu_button.set_popover(Some(&build_popover_menu(
        &menu_button, /* , &unit_stack*/
    )));

    let title_bar = gtk::HeaderBar::builder().build();

    title_bar.pack_start(&menu_button);

    /*    let right_bar = gtk::HeaderBar::builder().hexpand(true)
    .build(); */

    let right_bar_label = gtk::Label::builder()
        .label("Service Name")
        .attributes(&{
            let attribute_list = AttrList::new();
            attribute_list.insert(AttrInt::new_weight(Weight::Bold));
            attribute_list
        })
        .build();

    /*         let gtk_box_test = gtk::Box::new(Orientation::Horizontal, 0);
    gtk_box_test.append(&right_bar_label);
    gtk_box_test.set_width_request(100); */
    title_bar.pack_start(&right_bar_label);

    let action_buttons = gtk::Box::new(Orientation::Horizontal, 0);

    action_buttons.append(&{
        gtk::Label::builder()
            .label("Enabled:")
            .attributes(&{
                let attribute_list = AttrList::new();
                attribute_list.insert(AttrInt::new_weight(Weight::Bold));
                attribute_list
            })
            .build()
    });

    let ablement_switch = gtk::Switch::builder().focusable(true).build();

    action_buttons.append(&ablement_switch);

    let start_button = gtk::Button::builder()
        .hexpand(true)
        .label("Start")
        .focusable(true)
        .receives_default(true)
        .build();
    action_buttons.append(&start_button);

    let stop_button = gtk::Button::builder()
        .hexpand(true)
        .label("Stop")
        .focusable(true)
        .receives_default(true)
        .build();
    action_buttons.append(&stop_button);

    title_bar.pack_end(&action_buttons);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(application)
        .title("SystemD Manager")
        .default_height(600)
        .default_width(1000)
        .child(&main_box)
        .titlebar(&title_bar)
        .build();

    /*     let services_ = systemd::collect_togglable_services(&unit_files);
    let services_ref = Rc::new(services_);
    fill_sysd_unit_list(
        &services_list,
        &services_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    let sockets_ = systemd::collect_togglable_sockets(&unit_files);
    let sockets_ref = Rc::new(sockets_);
    fill_sysd_unit_list(
        &sockets_list,
        &sockets_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    let timer_ = systemd::collect_togglable_timers(&unit_files);
    let timers_ref = Rc::new(timer_);
    fill_sysd_unit_list(
        &timers_list,
        &timers_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    ); */

    {
        fn handle_switch(
            unit_list: &gtk::ColumnView,
            // unit_ref: Rc<Vec<LoadedUnit>>,
            enabled: bool,
            switch: &gtk::Switch,
        ) {
            if let Some(model) = unit_list.model() {
                let Some(single_selection_model) = model.downcast_ref::<SingleSelection>() else {
                    panic!("Can't downcast to SingleSelection")
                };

                let Some(object) = single_selection_model.selected_item() else {
                    eprintln!("No selection objet");
                    return;
                };

                let box_any = match object.downcast::<BoxedAnyObject>() {
                    Ok(any_objet) => any_objet,
                    Err(val) => {
                        eprintln!("Selection Error: {:?}", val);
                        return;
                    }
                };

                let unit: Ref<LoadedUnit> = box_any.borrow();

                let status =
                    systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);
                let is_unit_enable = status == EnablementStatus::Enabled;

                if enabled && !is_unit_enable {
                    if let Ok(_) = systemd::enable_unit_files(&unit) {
                        switch.set_state(true);
                    }
                } else if !enabled && is_unit_enable {
                    if let Ok(_) = systemd::disable_unit_files(&unit) {
                        switch.set_state(false);
                    }
                }
            }
        }
        let column_view = column_view.clone();
        ablement_switch.connect_state_set(move |switch, enabled| {
            handle_switch(&column_view, /*unit_ref,*/ enabled, switch);
            Propagation::Proceed
        });

        /*         ablement_switch.connect_state_set(move |switch, enabled| {
            let (unit_listbox, unit_ref) = match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => (&services_list, services_ref.clone()),
                "Sockets" => (&sockets_list, sockets_ref.clone()),
                "Timers" => (&timers_list, timers_ref.clone()),
                _ => unreachable!(),
            };

            handle_switch(unit_listbox, unit_ref, enabled, switch);
            Propagation::Proceed
        }); */
    }

    {
        // NOTE: Journal Refresh Button
        let refresh_button = refresh_log_button.clone();
        let unit_journal = unit_journal_view.clone();
        let column_view = column_view.clone();
        refresh_button.connect_clicked(move |_| {
            let box_any = get_selected_unit!(column_view);
            let unit: Ref<LoadedUnit> = box_any.borrow();
            update_journal(&unit_journal, &unit);
        });
    }

    {
        // NOTE: Implement the start button
        let column_view = column_view.clone();
        start_button.connect_clicked(move |_| {
            let box_any = get_selected_unit!(column_view);
            let unit: Ref<LoadedUnit> = box_any.borrow();

            match systemd::start_unit(&unit) {
                Ok(()) => {
                    eprintln!("Unit {} started!", unit.primary())
                }
                Err(e) => eprintln!("Cant't start the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    {
        let column_view = column_view.clone();
        stop_button.connect_clicked(move |_| {
            let box_any = get_selected_unit!(column_view);
            let unit: Ref<LoadedUnit> = box_any.borrow();

            match systemd::stop_unit(&unit) {
                Ok(()) => {
                    eprintln!("Unit {} stopped!", unit.primary())
                }
                Err(e) => eprintln!("Cant't stop the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    {
        // NOTE: Save Button
        let unit_info = unit_info.clone();
        let column_view = column_view.clone();
        save_unit_file_button.connect_clicked(move |_| {
            let buffer = unit_info.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true);
            let box_any = get_selected_unit!(column_view);
            let unit: Ref<LoadedUnit> = box_any.borrow();

            systemd::save_text_to_file(&unit, &text);
        });
    }
    {
        let unit_info = unit_info.clone();
        let ablement_switch = ablement_switch.clone();
        let unit_journal = unit_journal_view.clone();
        let header = right_bar_label.clone();

        columnview_selection_model.connect_selected_item_notify(move |single_selection| {
            let Some(object) = single_selection.selected_item() else {
                eprint!("No object seletected");
                return;
            };

            let box_any = match object.downcast::<BoxedAnyObject>() {
                Ok(any_objet) => any_objet,
                Err(val) => {
                    eprintln!("Selection Error: {:?}", val);
                    return;
                }
            };

            let unit: Ref<LoadedUnit> = box_any.borrow();

            let description = systemd::get_unit_info(&unit);
            unit_info.buffer().set_text(&description);
            ablement_switch.set_active(
                // systemd::get_unit_file_state(sysd_unit)
                systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown)
                    == EnablementStatus::Enabled,
            );
            ablement_switch.set_state(ablement_switch.is_active());

            update_journal(&unit_journal, &unit);
            header.set_label(unit.display_name());
            println!("Unit {:#?}", unit);
        });
    }
    window.present();



    /*     // Quit the program when the program has been exited
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Define custom actions on keypress
    window.connect_key_press_event(move |_, key| {
        if let Key::Escape = key.get_keyval() {
            gtk::main_quit()
        }
        gtk::Inhibit(false)
    });

    gtk::main(); */
}
