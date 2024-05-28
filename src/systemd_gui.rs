use gtk::glib::property::PropertyGet;
use gtk::prelude::*;
use gtk::{self, SelectionModel};
use systemd::analyze::Analyze;
use systemd::dbus;
use systemd::dbus::UnitState;

//use gdk::Key;

use crate::systemd::dbus::SystemdUnit;

use self::pango::{AttrInt, AttrList};
use gtk::glib::types::Type;
use gtk::glib::{self, BoxedAnyObject, Propagation};

use gtk::pango::{self, Weight};
//use self::gio;
use gtk::{Application, ApplicationWindow, Orientation};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

// ANCHOR: main
const APP_ID: &str = "org.systemd.manager";

const ICON_YES: &str = "object-select-symbolic";
const ICON_NO: &str = "window-close-symbolic";

/// Updates the status icon for the selected unit
fn update_icon(icon: &gtk::Image, state: bool) {
    if state {
        icon.set_from_icon_name(Some(ICON_YES));
    } else {
        icon.set_from_icon_name(Some(ICON_NO));
    }
}

/// Create a `gtk::ListboxRow` and add it to the `gtk::ListBox`, and then add the `gtk::Image` to a vector so that we can later modify
/// it when the state changes.
fn create_row(path: &Path, state: UnitState, state_icons: &mut Vec<gtk::Image>) -> gtk::ListBoxRow {
    let filename = path.file_stem().unwrap().to_str().unwrap();
    let unit_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let unit_label = gtk::Label::new(Some(filename));
    let image = if state == UnitState::Enabled {
        gtk::Image::from_icon_name(ICON_YES)
    } else {
        gtk::Image::from_icon_name(ICON_NO)
    };

    unit_box.append(&unit_label);
    unit_box.append(&image);

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

/// Use `systemd-analyze blame` to fill out the information for the Analyze `gtk::Stack`.
fn setup_systemd_analyze(builder: &gtk::Builder) {
    let analyze_tree: gtk::TreeView = builder.object("analyze_tree").unwrap();
    let analyze_store = gtk::ListStore::new(&[Type::U32, Type::STRING]);

    // A simple macro for adding a column to the preview tree.
    macro_rules! add_column {
        ($preview_tree:ident, $title:expr, $id:expr) => {{
            let column = gtk::TreeViewColumn::new();
            let renderer = gtk::CellRendererText::new();
            column.set_title($title);
            column.set_resizable(true);
            column.pack_start(&renderer, true);
            column.add_attribute(&renderer, "text", $id);
            analyze_tree.append_column(&column);
        }};
    }

    add_column!(analyze_store, "Time (ms)", 0);
    add_column!(analyze_store, "Unit", 1);

    let units = Analyze::blame();

    for value in units.clone() {
        //println!("value time : {:?} serrvice {:?}", value.time, value.service);
        //analyze_store.insert_with_values(None, &[(value.time, &value.service)]);
        analyze_store.insert_with_values(None, &[(0, &value.time)]);
        analyze_store.insert_with_values(None, &[(1, &value.service)]);
    }

    analyze_tree.set_model(Some(&analyze_store));

    let total_time_label: gtk::Label = builder.object("time_to_boot").unwrap();
    let time = (units.iter().last().unwrap().time as f32) / 1000f32;
    total_time_label.set_label(format!("{} seconds", time).as_str());
}

struct TableRow {
    col1: u32,
    col2: String,
}

//https://github.com/gtk-rs/gtk4-rs/blob/master/examples/column_view_datagrid/main.rs
fn setup_systemd_analyze_tree(total_time_label: &gtk::Label) -> gtk::ColumnView {
    let store = gtk::gio::ListStore::new::<BoxedAnyObject>();

    let units = Analyze::blame();

    for value in units.clone() {
        println!("Analyse Tree Blame {:?}", value);
        store.append(&BoxedAnyObject::new(TableRow {
            col1: value.time,
            col2: value.service,
        }));
    }

    let single_selection = gtk::SingleSelection::new(Some(store));
    let analyze_tree = gtk::ColumnView::builder()
        .focusable(true)
        .model(&single_selection)
        .build();

    let col1factory = gtk::SignalListItemFactory::new();
    let col2factory = gtk::SignalListItemFactory::new();

    let col1 = gtk::ColumnViewColumn::new(Some("Time (ms)"), Some(col1factory));
    let col2 = gtk::ColumnViewColumn::new(Some("Unit"), Some(col2factory));
    analyze_tree.append_column(&col1);
    analyze_tree.append_column(&col2);

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

fn fill_service_list(
    services_list: &gtk::ListBox,
    unit_files: &Vec<SystemdUnit>,
    unit_info: &gtk::TextView,
    ablement_switch: &gtk::Switch,
    unit_journal: &gtk::TextView,
    right_header: &gtk::Label,
) -> Vec<SystemdUnit> {
    // NOTE: Services
    let services = dbus::collect_togglable_services(&unit_files);
    let mut services_icons = Vec::new();
    for service in services.clone() {
        let unit_row = create_row(
            Path::new(service.name.as_str()),
            service.state,
            &mut services_icons,
        );
        services_list.append(&unit_row);
    }

    {
        let services = services.clone();
        let services_list = services_list.clone();
        let unit_info = unit_info.clone();
        let ablement_switch = ablement_switch.clone();
        let unit_journal = unit_journal.clone();
        let header = right_header.clone();
        services_list.connect_row_selected(move |_, row| {
            let index = row.clone().unwrap().index();
            let service = &services[index as usize];
            let description = get_unit_info(service);
            unit_info.buffer().set_text(description.as_str());
            ablement_switch.set_active(dbus::get_unit_file_state(service.name.as_str()));
            ablement_switch.set_state(ablement_switch.is_active());
            update_journal(&unit_journal, service.name.as_str());
            header.set_label(get_filename(service.name.as_str()));
        });
    }

    services
}

fn fill_sokects_list(
    sockets_list: &gtk::ListBox,
    unit_files: &Vec<SystemdUnit>,
    unit_info: &gtk::TextView,
    ablement_switch: &gtk::Switch,
    unit_journal: &gtk::TextView,
    right_header: &gtk::Label,
) -> Vec<SystemdUnit> {
    let sockets = dbus::collect_togglable_sockets(&unit_files);
    let mut sockets_icons = Vec::new();
    for socket in sockets.clone() {
        let unit_row = create_row(
            Path::new(socket.name.as_str()),
            socket.state,
            &mut sockets_icons,
        );
        sockets_list.append(&unit_row);
    }

    {
        let sockets = sockets.clone();
        let sockets_list = sockets_list.clone();
        let unit_info = unit_info.clone();
        let ablement_switch = ablement_switch.clone();
        let unit_journal = unit_journal.clone();
        let header = right_header.clone();
        sockets_list.connect_row_selected(move |_, row| {
            let index = row.clone().unwrap().index();
            let socket = &sockets[index as usize];
            let description = get_unit_info(socket);
            unit_info.buffer().set_text(description.as_str());
            ablement_switch.set_active(dbus::get_unit_file_state(socket.name.as_str()));
            ablement_switch.set_state(true);
            update_journal(&unit_journal, socket.name.as_str());
            header.set_label(get_filename(socket.name.as_str()));
        });
    }
    sockets
}

fn fill_timers_list(
    timers_list: &gtk::ListBox,
    unit_files: &Vec<SystemdUnit>,
    unit_info: &gtk::TextView,
    ablement_switch: &gtk::Switch,
    unit_journal: &gtk::TextView,
    right_header: &gtk::Label,
) -> Vec<SystemdUnit> {
    let timers = dbus::collect_togglable_timers(&unit_files);
    let mut timers_icons = Vec::new();
    for timer in timers.clone() {
        let unit_row = create_row(
            Path::new(timer.name.as_str()),
            timer.state,
            &mut timers_icons,
        );

        timers_list.append(&unit_row);
    }

    {
        let timers = timers.clone();
        let timers_list = timers_list.clone();
        let unit_info = unit_info.clone();
        let ablement_switch = ablement_switch.clone();
        let unit_journal = unit_journal.clone();
        let header = right_header.clone();
        timers_list.connect_row_selected(move |_, row| {
            let index = row.clone().unwrap().index();
            let timer = &timers[index as usize];
            let description = get_unit_info(timer);
            unit_info.buffer().set_text(description.as_str());
            ablement_switch.set_active(dbus::get_unit_file_state(timer.name.as_str()));
            ablement_switch.set_state(true);
            update_journal(&unit_journal, timer.name.as_str());
            header.set_label(get_filename(timer.name.as_str()));
        });
    }
    timers
}

fn build_ui(application: &Application) {
    // List of all unit files on the system
    let unit_files = dbus::list_unit_files();

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

    let save_button = gtk::Button::builder()
        .label("gtk-save")
        .focusable(true)
        .receives_default(true)
        .build();

    let unit_file_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    unit_file_box.append(&unit_file_stack_scrolled_window);
    unit_file_box.append(&save_button);

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

    let stack_switcher = gtk::StackSwitcher::builder()
    .stack(&info_stack)
    .build();

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

    let services = fill_service_list(
        &services_list,
        &unit_files,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );
    let sockets = fill_sokects_list(
        &sockets_list,
        &unit_files,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );
    let timers = fill_timers_list(
        &timers_list,
        &unit_files,
        &unit_info,
        &ablement_switch,
        &unit_journal_view,
        &right_bar_label,
    );

    {
        // NOTE: Implement the {dis, en}able button
        let services = services.clone();
        let services_list = services_list.clone();
        let sockets = sockets.clone();
        let sockets_list = sockets_list.clone();
        let timers = timers.clone();
        let timers_list = timers_list.clone();
        let unit_stack = unit_stack.clone();
        ablement_switch.connect_state_set(move |switch, enabled| {
            match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = &services[index as usize];
                    let service_path = Path::new(service.name.as_str())
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    if enabled && !dbus::get_unit_file_state(service.name.as_str()) {
                        dbus::enable_unit_files(service_path);
                        switch.set_state(true);
                        Propagation::Proceed
                    } else if !enabled && dbus::get_unit_file_state(service.name.as_str()) {
                        dbus::disable_unit_files(service_path);
                        switch.set_state(false);
                        Propagation::Proceed
                    } else {
                        Propagation::Stop
                    }
                }
                "Sockets" => {
                    let index = sockets_list.selected_row().unwrap().index();
                    let socket = &sockets[index as usize];
                    let socket_path = get_filename(socket.name.as_str());
                    if enabled && !dbus::get_unit_file_state(socket.name.as_str()) {
                        dbus::enable_unit_files(socket_path);
                        switch.set_state(true);
                    } else if !enabled && dbus::get_unit_file_state(socket.name.as_str()) {
                        dbus::disable_unit_files(socket_path);
                        switch.set_state(false);
                    }
                    Propagation::Proceed
                }
                "Timers" => {
                    let index = timers_list.selected_row().unwrap().index();
                    let timer = &timers[index as usize];
                    let timer_path = Path::new(timer.name.as_str())
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    if enabled && !dbus::get_unit_file_state(timer.name.as_str()) {
                        dbus::enable_unit_files(timer_path);
                        switch.set_state(true);
                    } else if !enabled && dbus::get_unit_file_state(timer.name.as_str()) {
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
        let services = services.clone();
        let services_list = services_list.clone();
        let sockets = sockets.clone();
        let sockets_list = sockets_list.clone();
        let timers = timers.clone();
        let timers_list = timers_list.clone();
        let unit_stack = unit_stack.clone();
        let refresh_button = refresh_log_button.clone();
        let unit_journal = unit_journal_view.clone();
        refresh_button.connect_clicked(move |_| {
            match unit_stack.visible_child_name().unwrap().as_str() {
                "Services" => {
                    let index = services_list.selected_row().unwrap().index();
                    let service = &services[index as usize];
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

    //window.set_titlebar(None::<&gtk::Box>);
    /*
    gtk::init().unwrap_or_else(|_| panic!("tv-renamer: failed to initialize GTK."));

    let builder = gtk::Builder::from_string(include_str!("interface4.glade"));
    let window: gtk::Window = builder.object("main_window").unwrap();
    let unit_stack: gtk::Stack = builder.object("unit_stack").unwrap();
    let services_list: gtk::ListBox = builder.object("services_list").unwrap();
    let sockets_list: gtk::ListBox = builder.object("sockets_list").unwrap();
    let timers_list: gtk::ListBox = builder.object("timers_list").unwrap();
    let unit_info: gtk::TextView = builder.object("unit_info").unwrap();
    let ablement_switch: gtk::Switch = builder.object("ablement_switch").unwrap();
    let start_button: gtk::Button = builder.object("start_button").unwrap();
    let stop_button: gtk::Button = builder.object("stop_button").unwrap();
    let save_unit_file: gtk::Button = builder.object("save_button").unwrap();
    let unit_menu_label: gtk::Label = builder.object("unit_menu_label").unwrap();
    let unit_popover: gtk::PopoverMenu = builder.object("unit_menu_popover").unwrap();
    let services_button: gtk::Button = builder.object("services_button").unwrap();
    let sockets_button: gtk::Button = builder.object("sockets_button").unwrap();
    let timers_button: gtk::Button = builder.object("timers_button").unwrap();
    let unit_journal: gtk::TextView = builder.object("unit_journal_view").unwrap();
    let refresh_log_button: gtk::Button = builder.object("refresh_log_button").unwrap();
    let right_header: gtk::Label = builder.object("header_service_label").unwrap(); */
    /*
       {
           // NOTE: Services Menu Button
           let label = unit_menu_label.clone();
           let stack = unit_stack.clone();
           let popover = unit_popover.clone();
           services_button.connect_clicked(move |_| {
               stack.set_visible_child_name("Services");
               label.set_text("Services");
               popover.set_visible(false);
           });
       }

       {
           // NOTE: Sockets Menu Button
           let label = unit_menu_label.clone();
           let stack = unit_stack.clone();
           let popover = unit_popover.clone();
           sockets_button.connect_clicked(move |_| {
               stack.set_visible_child_name("Sockets");
               label.set_text("Sockets");
               popover.set_visible(false);
           });
       }

       {
           // NOTE: Timers Menu Button
           let label = unit_menu_label.clone();
           let stack = unit_stack.clone();
           let popover = unit_popover.clone();
           timers_button.connect_clicked(move |_| {
               stack.set_visible_child_name("Timers");
               label.set_text("Timers");
               popover.set_visible(false);
           });
       }

       // Setup the Analyze stack
       setup_systemd_analyze(&builder);

       // List of all unit files on the system
       let unit_files = dbus::list_unit_files();

       // NOTE: Services
       let services = dbus::collect_togglable_services(&unit_files);
       let mut services_icons = Vec::new();
       for service in services.clone() {
           let mut unit_row = gtk::ListBoxRow::new();
           create_row(
               &mut unit_row,
               Path::new(service.name.as_str()),
               service.state,
               &mut services_icons,
           );
           services_list.insert(&unit_row, -1);
       }

       {
           let services = services.clone();
           let services_list = services_list.clone();
           let unit_info = unit_info.clone();
           let ablement_switch = ablement_switch.clone();
           let unit_journal = unit_journal.clone();
           let header = right_header.clone();
           services_list.connect_row_selected(move |_, row| {
               let index = row.clone().unwrap().index();
               let service = &services[index as usize];
               let description = get_unit_info(service.name.as_str());
               unit_info.buffer().set_text(description.as_str());
               ablement_switch.set_active(dbus::get_unit_file_state(service.name.as_str()));
               ablement_switch.set_state(ablement_switch.is_active());
               update_journal(&unit_journal, service.name.as_str());
               header.set_label(get_filename(service.name.as_str()));
           });
       }

       // NOTE: Sockets
       let sockets = dbus::collect_togglable_sockets(&unit_files);
       let mut sockets_icons = Vec::new();
       for socket in sockets.clone() {
           let mut unit_row = gtk::ListBoxRow::new();
           create_row(
               &mut unit_row,
               Path::new(socket.name.as_str()),
               socket.state,
               &mut sockets_icons,
           );
           sockets_list.insert(&unit_row, -1);
       }

       {
           let sockets = sockets.clone();
           let sockets_list = sockets_list.clone();
           let unit_info = unit_info.clone();
           let ablement_switch = ablement_switch.clone();
           let unit_journal = unit_journal.clone();
           let header = right_header.clone();
           sockets_list.connect_row_selected(move |_, row| {
               let index = row.clone().unwrap().index();
               let socket = &sockets[index as usize];
               let description = get_unit_info(socket.name.as_str());
               unit_info.buffer().set_text(description.as_str());
               ablement_switch.set_active(dbus::get_unit_file_state(socket.name.as_str()));
               ablement_switch.set_state(true);
               update_journal(&unit_journal, socket.name.as_str());
               header.set_label(get_filename(socket.name.as_str()));
           });
       }

       // NOTE: Timers
       let timers = dbus::collect_togglable_timers(&unit_files);
       let mut timers_icons = Vec::new();
       for timer in timers.clone() {
           let mut unit_row = gtk::ListBoxRow::new();
           create_row(
               &mut unit_row,
               Path::new(timer.name.as_str()),
               timer.state,
               &mut timers_icons,
           );
           timers_list.insert(&unit_row, -1);
       }

       {
           let timers = timers.clone();
           let timers_list = timers_list.clone();
           let unit_info = unit_info.clone();
           let ablement_switch = ablement_switch.clone();
           let unit_journal = unit_journal.clone();
           let header = right_header.clone();
           timers_list.connect_row_selected(move |_, row| {
               let index = row.clone().unwrap().index();
               let timer = &timers[index as usize];
               let description = get_unit_info(timer.name.as_str());
               unit_info.buffer().set_text(description.as_str());
               ablement_switch.set_active(dbus::get_unit_file_state(timer.name.as_str()));
               ablement_switch.set_state(true);
               update_journal(&unit_journal, timer.name.as_str());
               header.set_label(get_filename(timer.name.as_str()));
           });
       }

       {
           // NOTE: Implement the {dis, en}able button
           let services = services.clone();
           let services_list = services_list.clone();
           let sockets = sockets.clone();
           let sockets_list = sockets_list.clone();
           let timers = timers.clone();
           let timers_list = timers_list.clone();
           let unit_stack = unit_stack.clone();
           /* ablement_switch.connect_state_set(move |switch, enabled| {
               match unit_stack.visible_child_name().unwrap().as_str() {
                   "Services" => {
                       let index = services_list.selected_row().unwrap().index();
                       let service = &services[index as usize];
                       let service_path = Path::new(service.name.as_str())
                           .file_name()
                           .unwrap()
                           .to_str()
                           .unwrap();
                       if enabled && !dbus::get_unit_file_state(service.name.as_str()) {
                           dbus::enable_unit_files(service_path);
                           switch.set_state(true);

                       } else if !enabled && dbus::get_unit_file_state(service.name.as_str()) {
                           dbus::disable_unit_files(service_path);
                           switch.set_state(false);
                       }
                   }
                   "Sockets" => {
                       let index = sockets_list.selected_row().unwrap().index();
                       let socket = &sockets[index as usize];
                       let socket_path = get_filename(socket.name.as_str());
                       if enabled && !dbus::get_unit_file_state(socket.name.as_str()) {
                           dbus::enable_unit_files(socket_path);
                           switch.set_state(true);
                       } else if !enabled && dbus::get_unit_file_state(socket.name.as_str()) {
                           dbus::disable_unit_files(socket_path);
                           switch.set_state(false);
                       }
                   }
                   "Timers" => {
                       let index = timers_list.selected_row().unwrap().index();
                       let timer = &timers[index as usize];
                       let timer_path = Path::new(timer.name.as_str())
                           .file_name()
                           .unwrap()
                           .to_str()
                           .unwrap();
                       if enabled && !dbus::get_unit_file_state(timer.name.as_str()) {
                           dbus::enable_unit_files(timer_path);
                           switch.set_state(true);
                       } else if !enabled && dbus::get_unit_file_state(timer.name.as_str()) {
                           dbus::disable_unit_files(timer_path);
                           switch.set_state(false);
                       }
                   }
                   _ => unreachable!(),
               }
               //gtk::Inhibit(true)
           }); */
       }

       {
           // NOTE: Implement the start button
           let services = services.clone();
           let services_list = services_list.clone();
           let sockets = sockets.clone();
           let sockets_list = sockets_list.clone();
           let timers = timers.clone();
           let timers_list = timers_list.clone();
           let services_icons = services_icons.clone();
           let sockets_icons = sockets_icons.clone();
           let timers_icons = timers_icons.clone();
           let unit_stack = unit_stack.clone();
           start_button.connect_clicked(move |_| {
               match unit_stack.visible_child_name().unwrap().as_str() {
                   "Services" => {
                       let index = services_list.selected_row().unwrap().index();
                       let service = &services[index as usize];
                       if let None = dbus::start_unit(
                           Path::new(service.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&services_icons[index as usize], true);
                       }
                   }
                   "Sockets" => {
                       let index = sockets_list.selected_row().unwrap().index();
                       let socket = &sockets[index as usize];
                       if let None = dbus::start_unit(
                           Path::new(socket.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&sockets_icons[index as usize], true);
                       }
                   }
                   "Timers" => {
                       let index = timers_list.selected_row().unwrap().index();
                       let timer = &timers[index as usize];
                       if let None = dbus::start_unit(
                           Path::new(timer.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&timers_icons[index as usize], true);
                       }
                   }
                   _ => (),
               }
           });
       }

       {
           // NOTE: Implement the stop button
           let services = services.clone();
           let services_list = services_list.clone();
           let sockets = sockets.clone();
           let sockets_list = sockets_list.clone();
           let timers = timers.clone();
           let timers_list = timers_list.clone();
           let services_icons = services_icons.clone();
           let sockets_icons = sockets_icons.clone();
           let timers_icons = timers_icons.clone();
           let unit_stack = unit_stack.clone();
           stop_button.connect_clicked(move |_| {
               match unit_stack.visible_child_name().unwrap().as_str() {
                   "Services" => {
                       let index = services_list.selected_row().unwrap().index();
                       let service = &services[index as usize];
                       if let None = dbus::stop_unit(
                           Path::new(service.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&services_icons[index as usize], false);
                       }
                   }
                   "Sockets" => {
                       let index = sockets_list.selected_row().unwrap().index();
                       let socket = &sockets[index as usize];
                       if let None = dbus::stop_unit(
                           Path::new(socket.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&sockets_icons[index as usize], false);
                       }
                   }
                   "Timers" => {
                       let index = timers_list.selected_row().unwrap().index();
                       let timer = &timers[index as usize];
                       if let None = dbus::stop_unit(
                           Path::new(timer.name.as_str())
                               .file_name()
                               .unwrap()
                               .to_str()
                               .unwrap(),
                       ) {
                           update_icon(&timers_icons[index as usize], false);
                       }
                   }
                   _ => (),
               }
           });
       }

       {
           // NOTE: Save Button
           let unit_info = unit_info.clone();
           let services = services.clone();
           let services_list = services_list.clone();
           let sockets = sockets.clone();
           let sockets_list = sockets_list.clone();
           let timers = timers.clone();
           let timers_list = timers_list.clone();
           let unit_stack = unit_stack.clone();
           save_unit_file.connect_clicked(move |_| {
               let buffer = unit_info.buffer();
               let start = buffer.start_iter();
               let end = buffer.end_iter();
               let text = buffer.text(&start, &end, true);
               let path = match unit_stack.visible_child_name().unwrap().as_str() {
                   "Services" => {
                       &services[services_list.selected_row().unwrap().index() as usize].name
                   }
                   "Sockets" => &sockets[sockets_list.selected_row().unwrap().index() as usize].name,
                   "Timers" => &timers[timers_list.selected_row().unwrap().index() as usize].name,
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

       
    */
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
