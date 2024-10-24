use adw::Toast;
use gtk::pango::{self, Weight};
use gtk::{gdk, Orientation};
use gtk::{gio, prelude::*, SingleSelection};

use crate::systemd::enums::{ActiveState, EnablementStatus, UnitType};
use crate::widget::button_icon::ButtonIcon;
use crate::widget::journal::JournalPanel;
use crate::widget::preferences::data::PREFERENCES;
use crate::widget::preferences::PreferencesDialog;
use crate::widget::unit_file_panel::UnitFilePanel;
use crate::widget::unit_info::UnitInfoPanel;
use crate::widget::{self, title_bar};
use log::{debug, error, info, warn};

use crate::systemd;
use crate::widget::menu_button::ExMenuButton;
use systemd::data::UnitInfo;

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use std::cell::RefMut;
use std::rc::Rc;
use std::sync::{LazyLock, RwLock};

use crate::info::rowitem;
use strum::IntoEnumIterator;

pub const APP_ID: &str = "io.github.plrigaux.sysd-manager";

pub static SELECTED_UNIT: LazyLock<RwLock<Option<UnitInfo>>> = LazyLock::new(|| RwLock::new(None));

#[macro_export]
macro_rules! get_selected_unit_old {
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

macro_rules! selected_unit {
    ($closure:expr) => {{
        let unit_read = match SELECTED_UNIT.read() {
            Ok(unit_read) => unit_read,
            Err(e) => {
                warn!("Failed to unlock unit {:?}", e);
                return;
            }
        };

        let Some(unit_ref) = &*unit_read else {
            error!("No selected unit");
            return;
        };

        $closure(&unit_ref)
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

pub fn launch() -> glib::ExitCode {
    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|app| {
        load_css();
        title_bar::on_startup(app)
    });
    app.connect_activate(build_ui);

    app.run()
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/io/github/plrigaux/sysd-manager/style.css");

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(application: &adw::Application) {

    let store = gtk::gio::ListStore::new::<UnitInfo>();

    fill_store(&store);

    let units_browser = gtk::ColumnView::builder()
        //.model(&columnview_selection_model)
        .focusable(true)
        .build();

    let col_unit_name_factory = gtk::SignalListItemFactory::new();
    let col_type_factory = gtk::SignalListItemFactory::new();
    let col_enable_factory = gtk::SignalListItemFactory::new();
    let col_active_state_factory = gtk::SignalListItemFactory::new();
    let col_description_factory = gtk::SignalListItemFactory::new();

    col_unit_name_factory.connect_setup(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_unit_name_factory.connect_bind(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        let v = entry.display_name();
        child.set_text(Some(&v));
    });

    col_type_factory.connect_setup(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_type_factory.connect_bind(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_text(Some(&entry.unit_type()));
    });

    col_enable_factory.connect_setup(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_enable_factory.connect_bind(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let child = item.child().and_downcast::<gtk::Inscription>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();

        let status_code: EnablementStatus = entry.enable_status().into();

        child.set_text(Some(status_code.to_str()));

        entry.bind_property("enable_status", &child, "text").build();
    });

    col_active_state_factory.connect_setup(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let image = gtk::Image::new();
        item.set_child(Some(&image));
    });

    col_active_state_factory.connect_bind(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let child = item.child().and_downcast::<gtk::Image>().unwrap();
        let entry = item.item().and_downcast::<UnitInfo>().unwrap();
        child.set_icon_name(Some(&entry.active_state_icon()));
        entry
            .bind_property("active_state_icon", &child, "icon-name")
            .build();
    });

    col_description_factory.connect_setup(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let row = gtk::Inscription::builder().xalign(0.0).build();
        item.set_child(Some(&row));
    });

    col_description_factory.connect_bind(move |_factory, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
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

    units_browser.append_column(&col1_unit);
    units_browser.append_column(&col2_unit_type);
    units_browser.append_column(&col3_enable_status);
    units_browser.append_column(&col4_active_state);
    units_browser.append_column(&col5_description);

    let sorter = units_browser.sorter();
    let store_sort = store.clone();
    let sort_model = gtk::SortListModel::new(Some(store_sort), sorter);
    let filtermodel =
        gtk::FilterListModel::new(Some(sort_model.clone()), None::<gtk::CustomFilter>);
    let columnview_selection_model = gtk::SingleSelection::new(Some(filtermodel.clone()));
    units_browser.set_model(Some(&columnview_selection_model));

    let unit_col_view_scrolled_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .focusable(true)
        .child(&units_browser)
        .build();

    let left_pane = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .width_request(350)
        .build();

    //-------------------------------------------
    let unit_file_panel = UnitFilePanel::new();
    let journal_panel = JournalPanel::new();

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

    let system_manager = adw::StyleManager::default();

    let is_dark = system_manager.is_dark();

    let unit_info_panel = UnitInfoPanel::new(is_dark);
    /*     let unit_analyse_scrolled_window = gtk::ScrolledWindow::builder()
           .vexpand(true)
           .focusable(true)
           .build();
    */
    let info_stack = gtk::Notebook::builder()
        .vexpand(true)
        //.transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    info_stack.append_page(&unit_info_panel, Some(&gtk::Label::new(Some("Unit Info"))));
    info_stack.append_page(&unit_file_panel, Some(&gtk::Label::new(Some("Unit File"))));
    info_stack.append_page(&journal_panel, Some(&gtk::Label::new(Some("Unit Journal"))));

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
        .valign(gtk::Align::BaselineCenter)
        .tooltip_text("To enable or disable a unit")
        .build();

    let toast_overlay = adw::ToastOverlay::new();
    {
        let column_view = units_browser.clone();
        let toast_overlay = toast_overlay.clone();
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
                set_switch_tooltip(enabled, switch, &unit.primary());
                return Propagation::Proceed;
            }

            let (enable_result, enable_status) = if enabled {
                (systemd::enable_unit_files(&unit), EnablementStatus::Enabled)
            } else {
                (
                    systemd::disable_unit_files(&unit),
                    EnablementStatus::Disabled,
                )
            };

            match enable_result {
                Ok(enablement_status_ret) => {
                    let toast_info = format!(
                        "New active statut ({}) for unit {}",
                        enablement_status_ret.to_string(),
                        unit.primary(),
                    );
                    info!("{toast_info}");

                    let toast = Toast::new(&toast_info);

                    toast_overlay.add_toast(toast);
                }

                Err(error) => {
                    let error_message = match error {
                        systemd::SystemdErrors::SystemCtlError(s) => s,
                        _ => format!("{:?}", error),
                    };
                    let toast_warn = format!(
                        "Action \"{:?}\" on unit \"{}\": FAILED! {:?}",
                        enable_status,
                        unit.primary(),
                        error_message
                    );
                    warn!("{toast_warn}");

                    let toast = Toast::new(&toast_warn);

                    toast_overlay.add_toast(toast);

                    //TODO put a timer to set back the switch

                    return Propagation::Stop;
                }
            }

            //let unit_file_state =
            //    systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);
            //info!("New Status : {:?}", unit_file_state);

            let enabled_new = enable_status == EnablementStatus::Enabled;
            switch.set_state(enabled_new);
            set_switch_tooltip(enabled, switch, &unit.primary());
            unit.set_enable_status(enable_status as u32);

            handle_switch_sensivity(enable_status, switch);

            Propagation::Proceed
        });
    }

    control_box.append(&ablement_switch);

    let start_button = ButtonIcon::new("Start", "media-playback-start-symbolic");
    control_box.append(&start_button);

    let stop_button = ButtonIcon::new("Stop", "process-stop");
    control_box.append(&stop_button);

    let restart_button = ButtonIcon::new("Restart", "view-refresh");
    control_box.append(&restart_button);

    {
        let toast_overlay = toast_overlay.clone();
        start_button.connect_clicked(move |_button| {
            let lambda = |unit: &UnitInfo| match systemd::start_unit(&unit) {
                Ok(_job) => {
                    let info = format!("Unit \"{}\" has been started!", unit.primary());

                    info!("{info}");

                    let toast = Toast::new(&info);
                    toast_overlay.add_toast(toast);

                    update_active_state(&unit, ActiveState::Active);
                }
                Err(e) => error!("Can't start the unit {}, because: {:?}", unit.primary(), e),
            };

            selected_unit!(lambda);
        });
    }

    {
        let toast_overlay = toast_overlay.clone();
        stop_button.connect_clicked(move |_button| {
            let lambda = |unit: &UnitInfo| match systemd::stop_unit(&unit) {
                Ok(_job) => {
                    let info = format!("Unit \"{}\" has been stopped!", unit.primary());
                    info!("{info}");
                    let toast = Toast::new(&info);
                    toast_overlay.add_toast(toast);

                    update_active_state(&unit, ActiveState::Inactive)
                }

                Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
            };

            selected_unit!(lambda);
        });
    }

    {
        let toast_overlay = toast_overlay.clone();
        restart_button.connect_clicked(move |_button| {
            let lambda = |unit: &UnitInfo| match systemd::restart_unit(&unit) {
                Ok(_job) => {
                    let info = format!("Unit \"{}\" has been restarted!", unit.primary());
                    info!("{info}");
                    let toast = Toast::new(&info);
                    toast_overlay.add_toast(toast);

                    update_active_state(&unit, ActiveState::Active);
                }
                Err(e) => error!("Can't stop the unit {}, because: {:?}", unit.primary(), e),
            };

            selected_unit!(lambda);
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

    let title_bar_elements = title_bar::build_title_bar(&search_bar);

    let search_entry = gtk::SearchEntry::new();
    search_entry.set_hexpand(true);

    let search_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .build();

    let mut filter_button_unit_type = ExMenuButton::new("Type");
    let mut filter_button_status = ExMenuButton::new("Enablement");
    let mut filter_button_active = ExMenuButton::new("Active");

    for unit_type in UnitType::iter().filter(|x| match *x {
        UnitType::Unknown(_) => false,
        _ => true,
    }) {
        filter_button_unit_type.add_item(unit_type.to_str());
    }

    for status in EnablementStatus::iter().filter(|x| match *x {
        EnablementStatus::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        filter_button_status.add_item(status.to_str());
    }

    for status in ActiveState::iter().filter(|x| match *x {
        ActiveState::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        filter_button_active.add_item(status.label());
    }

    search_box.append(&search_entry);
    search_box.append(&filter_button_unit_type);
    search_box.append(&filter_button_status);
    search_box.append(&filter_button_active);

    search_bar.set_child(Some(&search_box));

    {
        let search_button = title_bar_elements.search_button.clone();
        search_entry.connect_search_started(move |_| {
            search_button.set_active(true);
        });
    }
    {
        let search_button = title_bar_elements.search_button.clone();
        search_entry.connect_stop_search(move |_| {
            search_button.set_active(false);
        });
    }

    {
        let entry1 = search_entry.clone();
        let unit_col_view_scrolled_window = unit_col_view_scrolled_window.clone();
        let custom_filter = {
            let filter_button_unit_type = filter_button_unit_type.clone();
            let filter_button_status = filter_button_status.clone();
            let filter_button_active = filter_button_active.clone();

            let custom_filter = gtk::CustomFilter::new(move |object| {
                let Some(unit) = object.downcast_ref::<UnitInfo>() else {
                    error!("some wrong downcast_ref {:?}", object);
                    return false;
                };

                let text = entry1.text();

                let unit_type = unit.unit_type();
                let enable_status :EnablementStatus = unit.enable_status().into();
                let active_state: ActiveState = unit.active_state().into();

                filter_button_unit_type.contains_value(&Some(unit_type))
                    && filter_button_status.contains_value(&Some(enable_status.to_str().to_owned()))
                    && if text.is_empty() {
                        true
                    } else {
                        unit.display_name().contains(text.as_str())
                    }
                    && filter_button_active.contains_value(&Some(active_state.to_string()))
            });

            custom_filter
        };

        filter_button_unit_type.set_filter(custom_filter.clone());
        filter_button_status.set_filter(custom_filter.clone());
        filter_button_active.set_filter(custom_filter.clone());

        filtermodel.set_filter(Some(&custom_filter));

        let last_filter_string = Rc::new(BoxedAnyObject::new(String::new()));

        search_entry.connect_search_changed(move |entry| {
            let text = entry.text();

            debug!("Search text \"{text}\"");

            let mut last_filter: RefMut<String> = last_filter_string.borrow_mut();

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
            custom_filter.changed(change_type);

            //FIXME when the filter become empty the colunm view display nothing until you click on it
            unit_col_view_scrolled_window.queue_draw(); //TODO investigate the need
        });
    }

    {
        let store = store.clone();

        title_bar_elements
            .refresh_button
            .connect_clicked(move |_button| {
                fill_store(&store);
            });
    }

    left_pane.append(&search_bar);
    left_pane.append(&unit_col_view_scrolled_window);

    let window = widget::app_window::AppWindow::new(application);
    //let window = widget::window::AppWindow::new(application);

/*     let toolbar_view = adw::ToolbarView::builder().content(&main_box).build();

    toast_overlay.set_child(Some(&toolbar_view));

    toolbar_view.add_top_bar(&title_bar_elements.header_bar);

    window.set_content(Some(&toast_overlay)); */

    {
        let system_manager = adw::StyleManager::default();
        let unit_file_panel = unit_file_panel.clone();
        system_manager.connect_dark_notify(move |a: &adw::StyleManager| {
            let is_dark = a.is_dark();
            info!("is dark {is_dark}");
            unit_file_panel.set_dark(is_dark);
        });
    }

    {
        //let unit_file_info = unit_file_info.clone();
        let ablement_switch = ablement_switch.clone();
        //let unit_journal = unit_journal_view.clone();
        let header_label = title_bar_elements.right_bar_label.clone();
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

            {
                let mut selected_unit = SELECTED_UNIT.write().unwrap();
                *selected_unit = Some(unit.clone());
            }

            unit_info_panel.display_unit_info(&unit);

            unit_file_panel.set_file_content(&unit);

            let ablement_status =
                systemd::get_unit_file_state(&unit).unwrap_or(EnablementStatus::Unknown);

            unit.set_enable_status(ablement_status as u32);
            ablement_switch.set_active(ablement_status == EnablementStatus::Enabled);
            ablement_switch.set_state(ablement_switch.is_active());

            handle_switch_sensivity(ablement_status, &ablement_switch);

            journal_panel.display_journal(&unit);
            header_label.set_label(&unit.display_name());
            debug!("Unit {:#?}", unit);

            unit_prop_store.remove_all();

         
            /*          let info_panel = unit_info::fill_data(&unit);
            unit_analyse_scrolled_window.set_child(Some(&info_panel)); */
        });
    }

    window.present();

    if PREFERENCES.is_app_first_connection() {
        info!("Is application first connection");

        let pdialog = PreferencesDialog::new();

        adw::prelude::AdwDialogExt::present(&pdialog, Some(&window));
    }
}

fn fill_store(store: &gio::ListStore) {
    let unit_files: Vec<UnitInfo> = match systemd::list_units_description_and_state() {
        Ok(map) => map.into_values().collect(),
        Err(_e) => vec![],
    };

    store.remove_all();

    for value in unit_files {
        store.append(&value);
    }
    info!("Unit list refreshed! list size {}", store.n_items())
}

fn set_switch_tooltip(enabled: bool, switch: &gtk::Switch, unit_name: &str) {
    let action_text = if enabled { "Disable" } else { "Enable" };

    let text = format!("{action_text} unit <b>{unit_name}</b>");

    switch.set_tooltip_markup(Some(&text));
}

/// Whether the widget responds to input.
fn handle_switch_sensivity(unit_file_state: EnablementStatus, switch: &gtk::Switch) {
    let sensitive = if unit_file_state == EnablementStatus::Enabled
        || unit_file_state == EnablementStatus::Disabled
    {
        true
    } else {
        switch.set_tooltip_text(None);
        false
    };

    switch.set_sensitive(sensitive);
}

fn update_active_state(unit: &UnitInfo, state: ActiveState) {
    unit.set_active_state(state as u32);
    unit.set_active_state_icon(state.icon_name().to_owned());
}
