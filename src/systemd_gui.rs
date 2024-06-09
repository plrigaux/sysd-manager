use gtk::prelude::*;
use gtk::{self, gio, SingleSelection};
use log::debug;
use log::error;

use crate::{icon_label_button, menu};

use crate::systemd;
use systemd::{EnablementStatus, LoadedUnit};

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};

use gtk::{Application, ApplicationWindow, Orientation};

use std::cell::{Ref, RefMut};
use std::rc::Rc;

// ANCHOR: main
const APP_ID: &str = "org.systemd.manager";

const _ICON_YES: &str = "object-select-symbolic";
const _ICON_NO: &str = "window-close-symbolic";

use crate::menu::rowitem;
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

macro_rules! create_column_filter {
    ($func:ident) => {{
        let col_sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let Some(box_any1) = obj1.downcast_ref::<BoxedAnyObject>() else {
                panic!("some wrong downcast_ref {:?}", obj1);
            };

            let unit1: Ref<LoadedUnit> = box_any1.borrow();

            let Some(box_any2) = obj2.downcast_ref::<BoxedAnyObject>() else {
                panic!("some wrong downcast_ref {:?}", obj2);
            };

            let unit2: Ref<LoadedUnit> = box_any2.borrow();

            unit1.$func().cmp(unit2.$func()).into()
        });
        col_sorter
    }};
}

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &LoadedUnit) {
    let journal_output = systemd::get_unit_journal(unit);
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
    /*
       let filtermodel = gtk::FilterListModel::new(Some(store.clone()), None::<gtk::CustomFilter>);
       let columnview_selection_model = gtk::SingleSelection::new(Some(filtermodel.clone()));
    */

    let column_view = gtk::ColumnView::builder()
        //.model(&columnview_selection_model)
        .focusable(true)
        .build();

    let col_unit_name_factory = gtk::SignalListItemFactory::new();
    let col_type_factory = gtk::SignalListItemFactory::new();
    let col_enable_factory = gtk::SignalListItemFactory::new();
    let col_active_state_factory = gtk::SignalListItemFactory::new();
    let col_description_factory = gtk::SignalListItemFactory::new();

    col_unit_name_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_unit_name_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<LoadedUnit> = entry.borrow();
        child.set_label(r.display_name());
    });

    col_type_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_type_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let unit: Ref<LoadedUnit> = entry.borrow();
        child.set_label(unit.unit_type());
    });

    col_enable_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_enable_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let r: Ref<LoadedUnit> = entry.borrow();
        child.set_label(r.enable_status());
    });

    col_active_state_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_active_state_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let unit: Ref<LoadedUnit> = entry.borrow();
        child.set_label(unit.active_state());
    });

    col_description_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Label::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_description_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Label>().unwrap();
        let entry = item.item().and_downcast::<BoxedAnyObject>().unwrap();
        let unit: Ref<LoadedUnit> = entry.borrow();
        child.set_label(unit.description());
    });

    let col1_unit_name_sorter = create_column_filter!(primary);
    let col1_unit = gtk::ColumnViewColumn::builder()
        .title("Unit")
        .factory(&col_unit_name_factory)
        .resizable(true)
        .sorter(&col1_unit_name_sorter)
        .fixed_width(140)
        .build();

    let col2_unit_type_sorter = create_column_filter!(unit_type);
    let col2_unit_type = gtk::ColumnViewColumn::builder()
        .title("Type")
        .factory(&col_type_factory)
        .sorter(&col2_unit_type_sorter)
        .resizable(true)
        .fixed_width(75)
        .build();

    let col3_enable_sorter = create_column_filter!(enable_status);
    let col3_enable_status = gtk::ColumnViewColumn::builder()
        .title("Enable\nstatus")
        .factory(&col_enable_factory)
        .sorter(&col3_enable_sorter)
        .expand(true)
        .fixed_width(75)
        .build();

    let col_sorter = create_column_filter!(active_state);

    let col4_active_state = gtk::ColumnViewColumn::builder()
        .title("Active\nstatus")
        .factory(&col_active_state_factory)
        .sorter(&col_sorter)
        .fixed_width(75)
        .build();

    let col5_unit = gtk::ColumnViewColumn::new(Some("Description"), Some(col_description_factory));

    column_view.append_column(&col1_unit);
    column_view.append_column(&col2_unit_type);
    column_view.append_column(&col3_enable_status);
    column_view.append_column(&col4_active_state);
    column_view.append_column(&col5_unit);

    let sorter = column_view.sorter();
    let sort_model = gtk::SortListModel::new(Some(store), sorter);
    let filtermodel =
        gtk::FilterListModel::new(Some(sort_model.clone()), None::<gtk::CustomFilter>);
    let columnview_selection_model = gtk::SingleSelection::new(Some(filtermodel.clone()));
    column_view.set_model(Some(&columnview_selection_model));

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

    let unit_prop_store = gio::ListStore::new::<rowitem::Metadata>();

    let no_selection = gtk::SingleSelection::new(Some(unit_prop_store.clone()));

    let unit_prop_list_box = gtk::ListBox::builder().build();

    unit_prop_list_box.bind_model(Some(&no_selection), |object| {
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
            .build();

        if long_text {
            l1.set_tooltip_text(Some(&meta.col1()));
        }

        let l2 = gtk::Label::new(Some(&meta.col2()));

        box_.append(&l1);
        box_.append(&l2);

        box_.upcast::<gtk::Widget>()
    });

    let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&unit_prop_list_box)
        .build();

    let info_stack = gtk::Notebook::builder()
        .vexpand(true)
        //.transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    info_stack.append_page(
        &unit_analyse_scrolled_window,
        Some(&gtk::Label::new(Some("Unit Info"))),
    );
    info_stack.append_page(&unit_file_box, Some(&gtk::Label::new(Some("Unit File"))));
    info_stack.append_page(
        &unit_journal_box,
        Some(&gtk::Label::new(Some("Unit Journal"))),
    );
    //info_stack.add_titled(&unit_analyse_box, Some("Analyze"), "Analyze");

    // let stack_switcher = gtk::StackSwitcher::builder().stack(&info_stack).build();

    let right_pane = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .hexpand(true)
        .build();

    let control_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(5)
        .hexpand(true)
        .build();

    control_box.append(&{
        gtk::Label::builder()
            .label("Enabled:")
            .attributes(&{
                let attribute_list = AttrList::new();
                attribute_list.insert(AttrInt::new_weight(Weight::Bold));
                attribute_list
            })
            .build()
    });

    let ablement_switch = gtk::Switch::builder()
    .focusable(true)
    .build();

    {
        fn handle_switch(
            column_view: &gtk::ColumnView,
            // unit_ref: Rc<Vec<LoadedUnit>>,
            enabled: bool,
            switch: &gtk::Switch,
        ) {
            if let Some(model) = column_view.model() {
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
    }

    control_box.append(&ablement_switch);

    let start_button = gtk::Button::builder()
        .hexpand(true)
        .label("Start")
        .focusable(true)
        .receives_default(true)
        .build();
    control_box.append(&start_button);

    let stop_button = gtk::Button::builder()
        .hexpand(true)
        .label("Stop")
        .focusable(true)
        .receives_default(true)
        .build();
    control_box.append(&stop_button);

    let restart_button = gtk::Button::builder()
        .hexpand(true)
        .label("Retart")
        .focusable(true)
        .receives_default(true)
        .build();
  
    control_box.append(&restart_button);

    let ilb = icon_label_button::IconLabelButton::new();
    //ilb.set_property("label","test");
    ilb.set_label_text("tihis is it");
    control_box.append(&ilb);
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

    // right_pane.append(&stack_switcher);
    right_pane.append(&control_box);
    right_pane.append(&info_stack);

    // ---------------------------------------------------

    let main_box = gtk::Box::new(Orientation::Horizontal, 5);
    main_box.append(&left_pane);
    main_box.append(&right_pane);

    let search_bar = gtk::SearchBar::builder()
        .valign(gtk::Align::Start)
        // .key_capture_widget(&window)
        .build();

    let (title_bar, right_bar_label, search_button) = build_title_bar(&search_bar);

    let entry = gtk::SearchEntry::new();
    entry.set_hexpand(true);
    search_bar.set_child(Some(&entry));

    {
        let search_button = search_button.clone();
        entry.connect_search_started(move |_| {
            search_button.set_active(true);
        });
    }
    {
        let search_button = search_button.clone();
        entry.connect_stop_search(move |_| {
            search_button.set_active(false);
        });
    }

    {
        let entry1 = entry.clone();
        let custom_filter = gtk::CustomFilter::new(move |object| {
            let Some(box_any) = object.downcast_ref::<BoxedAnyObject>() else {
                error!("some wrong downcast_ref {:?}", object);
                return false;
            };

            let unit: Ref<LoadedUnit> = box_any.borrow();
            let text = entry1.text();

            if text.is_empty() {
                return true;
            }

            unit.display_name().contains(text.as_str())
        });

        filtermodel.set_filter(Some(&custom_filter));

        let last_filter_string = Rc::new(BoxedAnyObject::new(String::new()));

        entry.connect_search_changed(move |entry| {
            let text = entry.text();

            let mut last_filter: RefMut<String> = last_filter_string.borrow_mut();

            let change_type = if text.is_empty() {
                gtk::FilterChange::LessStrict
            } else if text.len() > last_filter.len() && text.starts_with(last_filter.as_str()) {
                gtk::FilterChange::MoreStrict
            } else if text.len() < last_filter.len() && last_filter.starts_with(text.as_str()) {
                gtk::FilterChange::LessStrict
            } else {
                gtk::FilterChange::Different
            };

            debug!("cur {} prev {}", text, last_filter);
            last_filter.replace_range(.., text.as_str());
            custom_filter.changed(change_type);
        });
    }

    left_pane.append(&search_bar);
    left_pane.append(&unit_col_view_scrolled_window);

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
        let unit_prop_store = unit_prop_store.clone();

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

            unit_prop_store.remove_all();

            match systemd::fetch_system_unit_info(&unit) {
                Ok(map) => {
                    for (key, value) in map {
                        unit_prop_store.append(&rowitem::Metadata::new(key, value));
                    }
                }
                Err(e) => error!("Fail to retreive Unit info: {:?}", e),
            }
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

fn build_title_bar(search_bar: &gtk::SearchBar) -> (gtk::HeaderBar, gtk::Label, gtk::ToggleButton) {
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

    let search_button = gtk::ToggleButton::new();
    search_button.set_icon_name("system-search-symbolic");
    title_bar.pack_start(&search_button);

    title_bar.pack_start(&right_bar_label);

    search_button
        .bind_property("active", search_bar, "search-mode-enabled")
        .sync_create()
        .bidirectional()
        .build();

    (title_bar, right_bar_label, search_button)
}
