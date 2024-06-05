use gtk::{self, SingleSelection};

use gtk::prelude::*;
use log::debug;
use log::error;

use crate::menu;
use crate::systemd::get_unit_journal;

use crate::systemd;
use systemd::{ EnablementStatus, LoadedUnit};

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};

use gtk::{Application, ApplicationWindow, Orientation};

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
            error!("No selection objet");
            return;
        };

        let box_any = match object.downcast::<BoxedAnyObject>() {
            Ok(any_objet) => any_objet,
            Err(val) => {
                error!("Selection Error: {:?}", val);
                return;
            }
        };
        box_any
    }};
}

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &LoadedUnit) {
    let journal_output = get_unit_journal(unit);
    journal.buffer().set_text(&journal_output);
}

pub fn launch() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_startup(menu::on_startup);
    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(application: &Application) {
    // List of all unit files on the system
    let unit_files: Vec<LoadedUnit> = match systemd::list_units_description_and_state() {
        Ok(map) => map.into_values().collect(),
        Err(e) => {
            debug!("{:?}", e);
            vec![]
        }
    };

    let store = gtk::gio::ListStore::new::<BoxedAnyObject>();

    for value in unit_files.clone() {
        //debug!("Analyse Tree Blame {:?}", value);
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

    /*     let attribute_list = AttrList::new();
     attribute_list.insert(AttrInt::new_weight(Weight::Medium));
     let total_time_label = gtk::Label::builder()
         .label("seconds ...")
         .attributes(&attribute_list)
         .build();

     // Setup the Analyze stack
    // let analyze_tree = setup_systemd_analyze_tree(&total_time_label);

     let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
         .vexpand(true)
         .focusable(true)
         .child(&analyze_tree)
         .build();

         unit_analyse_box.append(&total_time_label);
     unit_analyse_box.append(&unit_analyse_scrolled_window); */

    let info_stack = gtk::Stack::builder()
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    info_stack.add_titled(&unit_file_box, Some("Unit File"), "Unit File");
    info_stack.add_titled(&unit_journal_box, Some("Unit Journal"), "Unit Journal");
    //info_stack.add_titled(&unit_analyse_box, Some("Analyze"), "Analyze");

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
    let title_bar = gtk::HeaderBar::builder().build();

    let menu_button = menu::build_menu();

    title_bar.pack_end(&menu_button);

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

    let restart_button = gtk::Button::builder()
        .hexpand(true)
        .label("Start")
        .focusable(true)
        .receives_default(true)
        .build();
    action_buttons.append(&restart_button);

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
                    error!("No selection objet");
                    return;
                };

                let box_any = match object.downcast::<BoxedAnyObject>() {
                    Ok(any_objet) => any_objet,
                    Err(val) => {
                        error!("Selection Error: {:?}", val);
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
                    error!("Unit {} started!", unit.primary())
                }
                Err(e) => error!("Cant't start the unit {}, because: {:?}", unit.primary(), e),
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
                    error!("Unit {} stopped!", unit.primary())
                }
                Err(e) => error!("Cant't stop the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    {
        let column_view = column_view.clone();
        restart_button.connect_clicked(move |_| {
            let box_any = get_selected_unit!(column_view);
            let unit: Ref<LoadedUnit> = box_any.borrow();

            match systemd::restart_unit(&unit) {
                Ok(()) => {
                    error!("Unit {} restarted!", unit.primary())
                }
                Err(e) => error!("Cant't stop the unit {}, because: {:?}", unit.primary(), e),
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
                    error!("Selection Error: {:?}", val);
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
            debug!("Unit {:#?}", unit);
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
