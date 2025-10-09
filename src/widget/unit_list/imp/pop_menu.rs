use gettextrs::pgettext;
use log::{debug, info, warn};

use gtk::{gdk::Rectangle, prelude::*};

use crate::{
    consts::{DESTRUCTIVE_ACTION, FLAT, SUGGESTED_ACTION},
    format2,
    systemd::{data::UnitInfo, enums::ActiveState},
    utils::palette::blue,
    widget::{InterPanelMessage, unit_list::UnitListPanel},
};

pub fn setup_popup_menu(
    units_browser: &gtk::ColumnView,
    filtered_list: &gtk::FilterListModel,
    unit_list_panel: &UnitListPanel,
) {
    let gesture = gtk::GestureClick::builder()
        .button(gtk::gdk::BUTTON_SECONDARY)
        .build();
    {
        let units_browser_clone: gtk::ColumnView = units_browser.clone();
        let filtered_list: gtk::FilterListModel = filtered_list.clone();
        let unit_list_panel = unit_list_panel.clone();
        gesture.connect_pressed(move |_gesture_click, n_press, x, y| {
            debug!("Pressed n_press: {n_press} x: {x} y: {y}");

            let Some(adjustement_offset) = units_browser_clone
                .vadjustment()
                .map(|adjustment| adjustment.value())
            else {
                warn!("Failed to retreive the adjusment heigth");
                return;
            };

            let adjusted_y = (adjustement_offset + y) as i32;

            let mut child_op = units_browser_clone.first_child();

            let mut header_height = 0;

            let mut line_id = -2;
            while let Some(ref child) = child_op {
                let child_name = child.type_().name();
                if child_name == "GtkColumnViewRowWidget" {
                    header_height = child.height();
                } else if child_name == "GtkColumnListView" {
                    line_id = retreive_row_id(child, adjusted_y, header_height);
                    break;
                }

                child_op = child.next_sibling();
            }

            debug!("Line id {line_id} list count {}", filtered_list.n_items());

            if line_id < 0 {
                warn!("some wrong line_no {line_id}");
                return;
            }

            let line_id = line_id as u32;
            if let Some(object) = filtered_list.item(line_id) {
                let unit = object.downcast_ref::<UnitInfo>().expect("Ok");
                info!("Pointing on Unit {}", unit.primary());
                menu_show(
                    &units_browser_clone,
                    unit,
                    x as i32,
                    y as i32,
                    &unit_list_panel,
                );
            } else if line_id >= filtered_list.n_items() {
                warn!(
                    "Line id: {line_id} is over or equals the number of lines {}",
                    filtered_list.n_items()
                );
            } else {
                warn!("Menu right click. Something wrong");
            }
        });
    }

    gesture
        .connect_released(|_g, n_press, x, y| debug!("Released n_press: {n_press} x: {x} y: {y}"));

    units_browser.add_controller(gesture);
}

/// This works, because, we assume that each line have same height
fn retreive_row_id(widget: &gtk::Widget, y: i32, header_height: i32) -> i32 {
    let mut child_op = widget.first_child();

    let mut row_height = -1;
    while let Some(child) = child_op {
        let w_type_name = child.type_().name();
        debug!("widget type name: {w_type_name}");
        if w_type_name == "GtkColumnViewRowWidget" {
            row_height = child.height();
            break;
        }

        child_op = child.next_sibling();
    }

    if row_height > 0 {
        debug!("y {y} header_height {header_height} row_height {row_height}");
        (y - header_height) / row_height
    } else {
        warn!("No valid row height {row_height}");
        -1
    }
}

#[derive(Copy, Clone, Debug)]
enum MenuAction {
    Start,
    Stop,
    Restart,
}

fn menu_show(
    units_browser: &gtk::ColumnView,
    unit: &UnitInfo,
    x: i32,
    y: i32,
    unit_list_panel: &UnitListPanel,
) {
    let pop_menu = gtk::Popover::builder()
        .pointing_to(&Rectangle::new(x, y, 1, 1))
        .autohide(true)
        .position(gtk::PositionType::Right)
        .build();

    pop_menu.set_parent(units_browser);

    let box_ = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    pop_menu.set_child(Some(&box_));

    let tooltip = pgettext("controls", "Start unit {}");

    create_menu_button(
        &box_,
        //Button label
        &pgettext("controls", "Start"),
        &tooltip,
        "media-playback-start-symbolic",
        unit,
        MenuAction::Start,
        unit_list_panel,
    );

    let tooltip = pgettext("controls", "Stop unit {}");
    create_menu_button(
        &box_,
        //Button label
        &pgettext("controls", "Stop"),
        &tooltip,
        "process-stop",
        unit,
        MenuAction::Stop,
        unit_list_panel,
    );

    let tooltip = pgettext("controls", "Restart unit {}");

    create_menu_button(
        &box_,
        //Button label
        &pgettext("controls", "Restart"),
        &tooltip,
        "view-refresh",
        unit,
        MenuAction::Restart,
        unit_list_panel,
    );

    pop_menu.popup();
}

fn create_menu_button(
    box_: &gtk::Box,
    label_name: &str,
    tooltip: &str,
    icon_name: &str,
    unit: &UnitInfo,
    action: MenuAction,
    unit_list_panel: &UnitListPanel,
) {
    let blue = blue(unit_list_panel.is_dark()).get_color();
    let unit_str = format!(
        "<span fgcolor='{}' font_family='monospace' size='larger' weight='bold'>{}</span>",
        blue,
        unit.primary()
    );

    let tooltip = format2!(tooltip, unit_str);

    let button = gtk::Button::builder()
        .child(
            &adw::ButtonContent::builder()
                .label(pgettext("controls", label_name))
                .icon_name(icon_name)
                .halign(gtk::Align::Start)
                .build(),
        )
        .css_classes([FLAT])
        .halign(gtk::Align::Fill)
        .tooltip_markup(tooltip)
        .build();

    let unit_list_panel = unit_list_panel.clone();

    match (action, unit.active_state()) {
        (MenuAction::Start, ActiveState::Inactive | ActiveState::Deactivating) => {
            println!("Action {SUGGESTED_ACTION:?}");
            button.remove_css_class(FLAT);
            button.add_css_class(SUGGESTED_ACTION);
        }
        (
            MenuAction::Stop,
            ActiveState::Active
            | ActiveState::Activating
            | ActiveState::Reloading
            | ActiveState::Refreshing,
        ) => button.add_css_class(DESTRUCTIVE_ACTION),

        _ => {}
    };

    let unit = unit.clone();
    button.connect_clicked(move |button| {
        let inter_message = match action {
            MenuAction::Start => InterPanelMessage::StartUnit(button, &unit),
            MenuAction::Stop => InterPanelMessage::StopUnit(button, &unit),
            MenuAction::Restart => InterPanelMessage::ReStartUnit(button, &unit),
        };
        unit_list_panel.button_action(&inter_message);
    });
    box_.append(&button);
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_derive_row_id() {
        let y = 44;
        let header_height = 10;
        let row_height = 20;

        let row = (y - header_height) / row_height;

        println!("{row}");

        let y = 20;

        let row = (y - header_height) / row_height;

        println!("{row}");
    }
}
