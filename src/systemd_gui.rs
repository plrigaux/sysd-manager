use gtk;

use gtk::gdk::Cursor;
use gtk::prelude::*;
use systemd::analyze::Analyze;



//use gdk::Key;

use crate::grid_cell::{Entry, GridCell};
use crate::systemd::dbus::{self, EnablementStatus, SystemdUnit};

use self::pango::{AttrInt, AttrList};
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};
//use self::gio;
use gtk::{Application, ApplicationWindow, Orientation};

use std::cell::Ref;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
// ANCHOR: main
const APP_ID: &str = "org.systemd.manager";

const ICON_YES: &str = "object-select-symbolic";
const ICON_NO: &str = "window-close-symbolic";

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
fn create_row(systemd_unit: &SystemdUnit, state_icons: &mut Vec<gtk::Image>) -> gtk::ListBoxRow {
    let unit_box = gtk::CenterBox::new();
    let unit_label = gtk::Label::new(Some(&systemd_unit.name));
    let image = if systemd_unit.state == EnablementStatus::Enabled {
        gtk::Image::from_icon_name(ICON_YES)
    } else {
        gtk::Image::from_icon_name(ICON_NO)
    };

    unit_box.set_start_widget(Some(&unit_label));
    unit_box.set_end_widget(Some(&image));

    let unit_row = gtk::ListBoxRow::builder().child(&unit_box).build();

    state_icons.push(image);

    unit_row
}

/// Read the unit file and return it's contents so that we can display it in the `gtk::TextView`.
fn get_unit_info(su: &SystemdUnit) -> String {
    let mut file = fs::File::open(&su.path).unwrap();
    let mut output = String::new();
    let _ = file.read_to_string(&mut output);
    output
}

struct TableRow {
    col1: u32,
    col2: String,
}

//https://github.com/gtk-rs/gtk4-rs/blob/master/examples/column_view_datagrid/main.rs

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze_tree(total_time_label: &gtk::Label) -> gtk::ColumnView {
    let store = gtk::gio::ListStore::new::<BoxedAnyObject>();

    let units = Analyze::blame();

    for value in units.clone() {
        //println!("Analyse Tree Blame {:?}", value);
        store.append(&BoxedAnyObject::new(TableRow {
            col1: value.time,
            col2: value.service,
        }));
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
        let r: Ref<TableRow> = entry.borrow();
        let ent = Entry {
            name: r.col1.to_string(),
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
        let r: Ref<TableRow> = entry.borrow();
        let ent = Entry {
            name: r.col2.to_string(),
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
fn update_journal(journal: &gtk::TextView, unit_path: &str) {
    journal
        .buffer()
        .set_text(get_unit_journal(unit_path).as_str());
}

/// Obtains the journal log for the given unit.
fn get_unit_journal(unit_path: &str) -> String {
    let log = String::from_utf8(
        Command::new("journalctl")
            .arg("-b")
            .arg("-u")
            .arg(Path::new(unit_path).file_stem().unwrap().to_str().unwrap())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    log.lines()
        .rev()
        .map(|x| x.trim())
        .fold(String::with_capacity(log.len()), |acc, x| acc + "\n" + x)
}

// TODO: Fix clippy error and start using this everywhere
fn get_filename<'a>(path: &'a str) -> &str {
    Path::new(path).file_name().unwrap().to_str().unwrap()
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

fn build_popover_menu(menu_button: &gtk::MenuButton, unit_stack: &gtk::Stack) -> gtk::PopoverMenu {
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
        let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone();
        services_button.connect_clicked(move |_| {
            stack.set_visible_child_name(SERVICES_TITLE);
            mb.set_label(SERVICES_TITLE);
            popover.set_visible(false);
        });
    }

    {
        let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone();
        sockets_button.connect_clicked(move |_| {
            stack.set_visible_child_name(SOCKETS_TITLE);
            mb.set_label(SOCKETS_TITLE);
            popover.set_visible(false);
        });
    }

    {
        let popover = unit_menu_popover.clone();
        let mb = menu_button.clone();
        let stack = unit_stack.clone();
        timers_button.connect_clicked(move |_| {
            stack.set_visible_child_name(TIMERS_TITLE);
            mb.set_label(TIMERS_TITLE);
            popover.set_visible(false);
        });
    }

    unit_menu_popover
}

fn fill_sysd_unit_list(
    units_list: &gtk::ListBox,
    services_list_ref: &Rc<Vec<SystemdUnit>>,
    unit_info: &gtk::TextView,
    ablement_switch: &gtk::Switch,
    unit_journal: &gtk::TextView,
    right_header: &gtk::Label,
) {
    // NOTE: Services

    let mut services_icons = Vec::new();
    for systemd_unit in services_list_ref.iter() {
        let unit_row = create_row(&systemd_unit, &mut services_icons);
        units_list.append(&unit_row);
    }

    {
        let services_list_ref = services_list_ref.clone();
        let services_list = units_list.clone();
        let unit_info = unit_info.clone();
        let ablement_switch = ablement_switch.clone();
        let unit_journal = unit_journal.clone();
        let header = right_header.clone();
        //let window = window.clone();
        let wait = Cursor::from_name("wait", None);
        services_list.connect_row_selected(move |service_list_box, row| {
            println!("Start connect_row_selected");
            service_list_box.set_cursor(wait.as_ref());

            match row {
                Some(list_row) => {
                    let index = list_row.index();
                    let sysd_unit = services_list_ref.get(index as usize).unwrap();
                    let description = get_unit_info(&sysd_unit);
                    unit_info.buffer().set_text(&description);
                    ablement_switch.set_active(
                        dbus::get_unit_file_state(sysd_unit) == EnablementStatus::Enabled
                    );
                    ablement_switch.set_state(ablement_switch.is_active());

                    update_journal(&unit_journal, sysd_unit.name.as_str());
                    header.set_label(get_filename(sysd_unit.name.as_str()));
                }
                None => {
                    println!("no row - tbc")
                }
            }

            service_list_box.set_cursor_from_name(None);
            println!("STOP connect_row_selected");
        });
    }
}

fn build_ui(application: &Application) {
    // List of all unit files on the system
    let mut unit_files: Vec<SystemdUnit> = dbus::list_unit_files();
    unit_files.sort_by_key(|unit| unit.name.to_lowercase());

    let services_list = gtk::ListBox::new();

    let services_viewport = gtk::Viewport::builder().child(&services_list).build();

    let services_window = gtk::ScrolledWindow::builder()
        .name(SERVICES_TITLE)
        .focusable(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&services_viewport)
        .build();

    let sockets_list = gtk::ListBox::new();

    let sockets_viewport = gtk::Viewport::builder().child(&sockets_list).build();

    let sockets_window = gtk::ScrolledWindow::builder()
        .name(SOCKETS_TITLE)
        .focusable(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&sockets_viewport)
        .build();

    let timers_list = gtk::ListBox::new();

    let timers_viewport = gtk::Viewport::builder().child(&timers_list).build();

    let timers_window = gtk::ScrolledWindow::builder()
        .name(TIMERS_TITLE)
        .focusable(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&timers_viewport)
        .build();

    let unit_stack = gtk::Stack::builder()
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    unit_stack.add_titled(&services_window, Some(SERVICES_TITLE), SERVICES_TITLE);
    unit_stack.add_titled(&sockets_window, Some(SOCKETS_TITLE), SOCKETS_TITLE);
    unit_stack.add_titled(&timers_window, Some(TIMERS_TITLE), TIMERS_TITLE);

    let left_pane = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();

    left_pane.append(&unit_stack);
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

    menu_button.set_popover(Some(&build_popover_menu(&menu_button, &unit_stack)));

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
    title_bar.pack_end(&right_bar_label);

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
    /*
    let size_group = gtk::SizeGroup::new(gtk::SizeGroupMode::None);
    size_group.add_widget(&left_bar);
    size_group.add_widget(&right_bar); */

    /*     let titlebar_box = gtk::Box::new(Orientation::Horizontal, 0);
    titlebar_box.append(&left_bar);
    titlebar_box.append(&right_bar); */

    /*     let size_group = gtk::SizeGroup::new(gtk::SizeGroupMode::None);
    size_group.add_widget(&left_bar);
    size_group.add_widget(&right_bar); */

    // Create a window
    let window = ApplicationWindow::builder()
        .application(application)
        .title("SystemD Manager")
        .default_height(600)
        .default_width(1000)
        .child(&main_box)
        .titlebar(&title_bar)
        .build();

    let services_ = dbus::collect_togglable_services(&unit_files);
    let services_ref = Rc::new(services_);
    fill_sysd_unit_list(
        &services_list,
        &services_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    let sockets_ = dbus::collect_togglable_sockets(&unit_files);
    let sockets_ref = Rc::new(sockets_);
    fill_sysd_unit_list(
        &sockets_list,
        &sockets_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    let timer_ = dbus::collect_togglable_timers(&unit_files);
    let timers_ref = Rc::new(timer_);
    fill_sysd_unit_list(
        &timers_list,
        &timers_ref,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    {
        // NOTE: Implement the {dis, en}able button
        let services_ref = services_ref.clone();
        let services_list = services_list.clone();
        let sockets_ref = sockets_ref.clone();
        let sockets_list = sockets_list.clone();
        let timer_ref = timers_ref.clone();
        let timers_list = timers_list.clone();
        let unit_stack = unit_stack.clone();
        ablement_switch.connect_state_set(move |switch, enabled| {
            match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = &services_ref.get(index as usize).unwrap();
                    let service_path = Path::new(service.name.as_str())
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    if enabled && dbus::get_unit_file_state(service) != EnablementStatus::Enabled {
                        dbus::enable_unit_files(service_path);
                        switch.set_state(true);
                        Propagation::Proceed
                    } else if !enabled && dbus::get_unit_file_state(service) == EnablementStatus::Enabled {
                        dbus::disable_unit_files(service_path);
                        switch.set_state(false);
                        Propagation::Proceed
                    } else {
                        Propagation::Stop
                    }
                }
                "Sockets" => {
                    let index = sockets_list.selected_row().unwrap().index();
                    let socket = &sockets_ref[index as usize];
                    let socket_path = get_filename(socket.name.as_str());
                    if enabled && dbus::get_unit_file_state(socket) != EnablementStatus::Enabled {
                        dbus::enable_unit_files(socket_path);
                        switch.set_state(true);
                    } else if !enabled && dbus::get_unit_file_state(socket) == EnablementStatus::Enabled {
                        dbus::disable_unit_files(socket_path);
                        switch.set_state(false);
                    }
                    Propagation::Proceed
                }
                "Timers" => {
                    let index = timers_list.selected_row().unwrap().index();
                    let timer = &timer_ref[index as usize];
                    let timer_path = Path::new(timer.name.as_str())
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    if enabled && dbus::get_unit_file_state(timer) != EnablementStatus::Enabled {
                        dbus::enable_unit_files(timer_path);
                        switch.set_state(true);
                    } else if !enabled && dbus::get_unit_file_state(timer) == EnablementStatus::Enabled {
                        dbus::disable_unit_files(timer_path);
                        switch.set_state(false);
                    }
                    Propagation::Proceed
                }
                _ => unreachable!(),
            }
            //gtk::Inhibit(true)
        });
    }

    {
        // NOTE: Journal Refresh Button
        let services_ref = services_ref.clone();
        let services_list = services_list.clone();
        let sockets = sockets_ref.clone();
        let sockets_list = sockets_list.clone();
        let timers = timers_ref.clone();
        let timers_list = timers_list.clone();
        let unit_stack = unit_stack.clone();
        let refresh_button = refresh_log_button.clone();
        let unit_journal = unit_journal_view.clone();
        refresh_button.connect_clicked(move |_| {
            match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = &services_ref.get(index as usize).unwrap();
                    update_journal(&unit_journal, service.name.as_str());
                }
                "Sockets" => {
                    let index = sockets_list.selected_row().unwrap().index();
                    let socket = &sockets[index as usize];
                    update_journal(&unit_journal, socket.name.as_str());
                }
                "Timers" => {
                    let index = timers_list.selected_row().unwrap().index();
                    let timer = &timers[index as usize];
                    update_journal(&unit_journal, timer.name.as_str());
                }
                _ => unreachable!(),
            }
        });
    }

    {
        // NOTE: Implement the start button
        let services_ref = services_ref.clone();
        let services_list = services_list.clone();
        let sockets_ref = sockets_ref.clone();
        let sockets_list = sockets_list.clone();
        let timers_ref = timers_ref.clone();
        let timers_list = timers_list.clone();
        /*         let services_icons = services_icons.clone();
        let sockets_icons = sockets_icons.clone();
        let timers_icons = timers_icons.clone(); */
        let unit_stack = unit_stack.clone();
        start_button.connect_clicked(move |_| {
            let unit_option = match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = services_ref.get(index as usize).unwrap();
                    Some((index, service, services_list.clone()))
                }
                "Sockets" => {
                    let index = sockets_list.selected_row().unwrap().index();
                    let socket = &sockets_ref[index as usize];
                    Some((index, socket, sockets_list.clone()))
                }
                "Timers" => {
                    let index = timers_list.selected_row().unwrap().index();
                    let timer = &timers_ref[index as usize];
                    Some((index, timer, timers_list.clone()))
                }
                _ => None,
            };

            change_status_icon(unit_option, ICON_YES, dbus::start_unit);
        });
    }

    {
        // NOTE: Implement the stop button
        let services_ref = services_ref.clone();
        let services_list = services_list.clone();
        let sockets_ref = sockets_ref.clone();
        let sockets_list = sockets_list.clone();
        let timers = timers_ref.clone();
        let timers_list = timers_list.clone();
        /*         let services_icons = services_icons.clone();
        let sockets_icons = sockets_icons.clone();
        let timers_icons = timers_icons.clone(); */
        let unit_stack = unit_stack.clone();
        stop_button.connect_clicked(move |_| {
            let unit_option = match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = services_ref.get(index as usize).unwrap();
                    Some((index, service, services_list.clone()))
                }
                "Sockets" => {
                    let index = sockets_list.selected_row().unwrap().index();
                    let socket = &sockets_ref[index as usize];
                    Some((index, socket, sockets_list.clone()))
                }
                "Timers" => {
                    let index = timers_list.selected_row().unwrap().index();
                    let timer = &timers[index as usize];
                    Some((index, timer, timers_list.clone()))
                }
                _ => None,
            };

            change_status_icon(unit_option, ICON_NO, dbus::stop_unit);
        });
    }

    {
        // NOTE: Save Button
        let unit_info = unit_info.clone();
        let services_ref = services_ref.clone();
        let services_list = services_list.clone();
        let sockets_ref = sockets_ref.clone();
        let sockets_list = sockets_list.clone();
        let timers_ref = timers_ref.clone();
        let timers_list = timers_list.clone();
        let unit_stack = unit_stack.clone();
        save_unit_file_button.connect_clicked(move |_| {
            let buffer = unit_info.buffer();
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true);
            let path = match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let service = services_ref
                        .get(services_list.selected_row().unwrap().index() as usize)
                        .unwrap();
                    &service.name
                }
                "Sockets" => {
                    &sockets_ref[sockets_list.selected_row().unwrap().index() as usize].name
                }
                "Timers" => &timers_ref[timers_list.selected_row().unwrap().index() as usize].name,
                _ => unreachable!(),
            };
            match fs::OpenOptions::new().write(true).open(&path) {
                Ok(mut file) => {
                    if let Err(message) = file.write(text.as_bytes()) {
                        println!("Unable to write to file: {:?}", message);
                    }
                }
                Err(message) => println!("Unable to open file: {:?}", message),
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

fn change_status_icon(
    unit_option: Option<(i32, &SystemdUnit, gtk::ListBox)>,
    icon_name: &str,
    callback: fn(&str) -> Option<String>,
) {
    if let Some((_, sysd_unit, selected_list_box)) = unit_option {
        let full_name = &sysd_unit.full_name();
        if let None = callback(full_name) {
            let list_row = selected_list_box.selected_row().unwrap();
            let widget: gtk::Widget = list_row.child().unwrap();
            match widget.downcast::<gtk::CenterBox>() {
                Ok(center_box) => {
                    let icon = gtk::Image::from_icon_name(icon_name);
                    center_box.set_end_widget(Some(&icon));
                }
                Err(_w) => println!("This row is not a CenterBox"),
            }
        }
    }
}
