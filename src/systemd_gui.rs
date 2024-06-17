use gtk::prelude::*;
use gtk::{self, gio, SingleSelection};
use log::{debug, error, info, warn};

use crate::menu;

use crate::systemd::{self, ActiveState};
use systemd::{data::UnitInfo, EnablementStatus};

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};

use gtk::{Application, ApplicationWindow, Orientation};

use std::cell::RefMut;
use std::rc::Rc;

const APP_ID: &str = "org.tool.sysd.manager";

use crate::menu::rowitem;

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

        let unit = match object.downcast::<UnitInfo>() {
            Ok(any_objet) => any_objet,
            Err(val) => {
                error!("Selection Error: {:?}", val);
                return;
            }
        };
        unit
    }};
}

macro_rules! create_column_filter {
    ($func:ident) => {{
        let col_sorter = gtk::CustomSorter::new(move |obj1, obj2| {
            let Some(unit1) = obj1.downcast_ref::<UnitInfo>() else {
                panic!("some wrong downcast_ref {:?}", obj1);
            };

            let Some(unit2) = obj2.downcast_ref::<UnitInfo>() else {
                panic!("some wrong downcast_ref {:?}", obj2);
            };

            unit1.$func().cmp(&unit2.$func()).into()
        });
        col_sorter
    }};
}

/// Updates the associated journal `TextView` with the contents of the unit's journal log.
fn update_journal(journal: &gtk::TextView, unit: &UnitInfo) {
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
    let unit_files: Vec<UnitInfo> = match systemd::list_units_description_and_state() {
        Ok(map) => map.into_values().collect(),
        Err(e) => {
            debug!("{:?}", e);
            vec![]
        }
    };

    let store = gtk::gio::ListStore::new::<UnitInfo>();

    for value in unit_files {
        //debug!("Analyse Tree Blame {:?}", value);
        store.append(&value);
    }

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
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_unit_name_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        let v = entry.display_name();
        child.set_text(Some(&v));
    });

    col_type_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_type_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_text(Some(&entry.unit_type()));
    });

    col_enable_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_enable_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_text(entry.enable_status().as_deref());

        entry.bind_property("enable_status", &child, "text").build();
    });

    col_active_state_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let image = gtk::Image::new();
        item.set_child(Some(&image));
    });

    col_active_state_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Image>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_icon_name(Some(&entry.active_state_icon()));
        entry
            .bind_property("active_state_icon", &child, "icon-name")
            .build();
    });

    col_description_factory.connect_setup(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_description_factory.connect_bind(move |_factory, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_text(Some(&entry.description()));
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
        //.expand(true)
        .resizable(true)
        .fixed_width(70)
        .build();

    let col_sorter = create_column_filter!(active_state);

    let col4_active_state = gtk::ColumnViewColumn::builder()
        .title("Active\nstatus")
        .factory(&col_active_state_factory)
        .sorter(&col_sorter)
        .fixed_width(75)
        .resizable(true)
        .build();

    let col5_description = gtk::ColumnViewColumn::builder()
        .title("Description")
        .factory(&col_description_factory)
        .expand(true)
        .resizable(true)
        .build();

    column_view.append_column(&col1_unit);
    column_view.append_column(&col2_unit_type);
    column_view.append_column(&col3_enable_status);
    column_view.append_column(&col4_active_state);
    column_view.append_column(&col5_description);

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
        .child(&build_icon_label("Save", "document-save"))
        .focusable(true)
        .receives_default(true)
        .build();

    let unit_file_label = gtk::Label::builder()
        .hexpand(true)
        .selectable(true)
        .xalign(0.0)
        .build();

    let unit_file_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    unit_file_box.append(&unit_file_stack_scrolled_window);

    let unit_file_bottom_box = gtk::Box::builder()
        .spacing(5)
        .orientation(Orientation::Horizontal)
        .build();

    unit_file_bottom_box.append(&unit_file_label);
    unit_file_bottom_box.append(&save_unit_file_button);

    unit_file_box.append(&unit_file_bottom_box);

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
        .vexpand(false)
        .build();

    {
        let column_view = column_view.clone();
        ablement_switch.connect_state_set(move |switch, enabled| {
            // handle_switch(&column_view, /*unit_ref,*/ enabled, switch);

            debug!(
                "active {} state {} new {enabled}",
                switch.is_active(),
                switch.state()
            );

            let Some(model) = column_view.model() else {
                warn!("No model");
                return Propagation::Proceed;
            };

            let Some(single_selection_model) = model.downcast_ref::<SingleSelection>() else {
                panic!("Can't downcast to SingleSelection")
            };

            let Some(object) = single_selection_model.selected_item() else {
                error!("No selection objet");
                return Propagation::Proceed;
            };

            let unit = match object.downcast::<UnitInfo>() {
                Ok(any_objet) => any_objet,
                Err(val) => {
                    error!("Selection Error: {:?}", val);
                    return Propagation::Proceed;
                }
            };

            let enabled_status: EnablementStatus = unit.enable_status().into();

            if enabled && enabled_status == EnablementStatus::Enabled
                || !enabled && enabled_status != EnablementStatus::Enabled
            {
                set_switch_tooltip(enabled, switch);
                return Propagation::Proceed;
            }

            let (enable_result, action) = if enabled {
                (systemd::enable_unit_files(&unit), "Enable")
            } else {
                (systemd::disable_unit_files(&unit), "Disable")
            };

            match enable_result {
                Ok(enablement_status_ret) => {
                    info!("New statut: {}", enablement_status_ret.to_string());
                }
                Err(e) => warn!(
                    "{action} unit \"{}\": FAILED!, reason : {:?}",
                    unit.primary(),
                    e
                ),
            }

            let unit_file_state =
                systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);
            info!("New Status : {:?}", unit_file_state);

            let enabled_new =  unit_file_state == EnablementStatus::Enabled;
            switch.set_active(enabled_new);
            set_switch_tooltip(enabled_new, switch);
            unit.set_enable_status(unit_file_state.to_string());

            handle_switch_sensivity(unit_file_state, switch);

            Propagation::Proceed
        });
    }

    control_box.append(&ablement_switch);

    let start_button = gtk::Button::builder()
        .hexpand(true)
        .child(&build_icon_label("Start", "media-playback-start-symbolic"))
        .focusable(true)
        .receives_default(true)
        .build();
    control_box.append(&start_button);

    let stop_button = gtk::Button::builder()
        .hexpand(true)
        .child(&build_icon_label("Stop", "process-stop"))
        .focusable(true)
        .receives_default(true)
        .build();
    control_box.append(&stop_button);

    let restart_button = gtk::Button::builder()
        .hexpand(true)
        .focusable(true)
        .receives_default(true)
        .child(&build_icon_label("Retart", "view-refresh"))
        .build();

    control_box.append(&restart_button);

    {
        // NOTE: Implement the start button
        let column_view = column_view.clone();
        start_button.connect_clicked(move |_button| {
            let unit = get_selected_unit!(column_view);

            match systemd::start_unit(&unit) {
                Ok(()) => {
                    info!("Unit \"{}\" has been started!", unit.primary());
                    update_active_state(&unit, ActiveState::Active);
                }
                Err(e) => error!("Can't start the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    {
        let column_view = column_view.clone();
        stop_button.connect_clicked(move |_button| {
            let unit = get_selected_unit!(column_view);

            match systemd::stop_unit(&unit) {
                Ok(()) => {
                    info!("Unit \"{}\" stopped!", unit.primary());
                    update_active_state(&unit, ActiveState::Inactive)
                }

                Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    {
        let column_view = column_view.clone();
        restart_button.connect_clicked(move |_| {
            let unit = get_selected_unit!(column_view);

            match systemd::restart_unit(&unit) {
                Ok(()) => {
                    info!("Unit {} restarted!", unit.primary());
                    update_active_state(&unit, ActiveState::Active);
                }
                Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
            }
        });
    }

    // right_pane.append(&stack_switcher);
    right_pane.append(&control_box);
    right_pane.append(&info_stack);

    // ---------------------------------------------------

    let main_box = gtk::Paned::new(Orientation::Horizontal);
    //let main_box = gtk::Box::new(Orientation::Horizontal, 5);

    main_box.set_start_child(Some(&left_pane));
    main_box.set_end_child(Some(&right_pane));

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
        let unit_col_view_scrolled_window = unit_col_view_scrolled_window.clone();
        let custom_filter = gtk::CustomFilter::new(move |object| {
            let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                error!("some wrong downcast_ref {:?}", object);
                return false;
            };

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
            unit_col_view_scrolled_window.queue_resize(); //TODO investigate the need
        });
    }

    left_pane.append(&search_bar);
    left_pane.append(&unit_col_view_scrolled_window);

    
    // Create a window
    let window: ApplicationWindow = ApplicationWindow::builder()
        .application(application)
        .title(menu::APP_TITLE)
        .default_height(600)
        .default_width(1200)
        .child(&main_box)
        .titlebar(&title_bar)
        .build();

        
    {
        // NOTE: Journal Refresh Button
        let refresh_button = refresh_log_button.clone();
        let unit_journal = unit_journal_view.clone();
        let column_view = column_view.clone();
        refresh_button.connect_clicked(move |_| {
            let unit = get_selected_unit!(column_view);
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
            let unit = get_selected_unit!(column_view);
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
                warn!("No object seletected");
                return;
            };

            let unit = match object.downcast::<UnitInfo>() {
                Ok(any_objet) => any_objet,
                Err(val) => {
                    error!("Selection Error: {:?}", val);
                    return;
                }
            };

            let description = systemd::get_unit_info(&unit);

            let fp = match unit.file_path() {
                Some(s) => s,
                None => "".to_owned(),
            };

            unit_file_label.set_label(&fp);

            unit_info.buffer().set_text(&description);

            let ablement_status =
                systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);

            unit.set_enable_status(ablement_status.to_string());
            ablement_switch.set_active(ablement_status == EnablementStatus::Enabled);
            ablement_switch.set_state(ablement_switch.is_active());

            handle_switch_sensivity(ablement_status, &ablement_switch);

            update_journal(&unit_journal, &unit);
            header.set_label(&unit.display_name());
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

fn set_switch_tooltip(enabled: bool, switch: &gtk::Switch) {
    let text = if enabled {
        "Disable unit "
    } else {
        "Enable unit"
    };
    switch.set_tooltip_text(Some(text));
}

fn handle_switch_sensivity(unit_file_state: EnablementStatus, switch: &gtk::Switch) {
    let sensitive = if unit_file_state == EnablementStatus::Enabled
        || unit_file_state == EnablementStatus::Disabled
    {
        true
    } else {
        false
    };

    switch.set_sensitive(sensitive);
    switch.set_tooltip_text(None);
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

fn build_icon_label(label_name: &str, icon_name: &str) -> gtk::Box {
    let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    box_container.set_halign(gtk::Align::Center);
    let label = gtk::Label::new(Some(label_name));
    let icon = gtk::Image::from_icon_name(icon_name);

    box_container.append(&icon);
    box_container.append(&label);

    box_container
}

fn update_active_state(unit: &UnitInfo, state: ActiveState) {
    unit.set_active_state(state as u32);
    unit.set_active_state_icon(state.icon_name().to_owned());
}
