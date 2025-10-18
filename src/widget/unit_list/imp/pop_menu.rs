use std::{cell::RefCell, rc::Rc};

use gettextrs::pgettext;
use log::{debug, info, warn};

use gtk::{gdk::Rectangle, prelude::*};

use crate::{
    consts::{DESTRUCTIVE_ACTION, FLAT, SUGGESTED_ACTION},
    format2,
    systemd::{data::UnitInfo, enums::EnablementStatus},
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
                warn!("Some wrongs line_no {line_id}");
                return;
            }

            let line_id = line_id as u32;
            if let Some(object) = filtered_list.item(line_id) {
                let unit = object.downcast_ref::<UnitInfo>().expect("Ok");
                info!(
                    "Pointing on Unit {} state {}",
                    unit.primary(),
                    unit.active_state()
                );
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
    debug!("widjet {widget:?}");

    let mut child_op = widget.first_child();

    debug!("child_op {child_op:?}");

    let mut row_height = -1;
    while let Some(child) = child_op {
        let w_type_name = child.type_().name();
        debug!("widget type name: {w_type_name}");
        if w_type_name == "GtkColumnViewRowWidget" {
            row_height = child.height();
            if row_height > 0 {
                break;
            }
        }

        child_op = child.next_sibling();
    }

    if row_height > 0 {
        debug!("y {y} header_height {header_height} row_height {row_height}");
        (y - header_height) / row_height
    } else {
        warn!("Not a valid row height {row_height}");
        -1
    }
}

#[derive(Copy, Clone, Debug)]
enum MenuAction {
    Start,
    Stop,
    Restart,
    Enable,
    Disable,
    /*     Mask,
    UnMask, */
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

    box_.append(
        &gtk::Label::builder()
            .label(unit.primary())
            .tooltip_text(unit.primary())
            .max_width_chars(20)
            .ellipsize(pango::EllipsizeMode::End)
            .css_classes(["heading"])
            .build(),
    );
    box_.append(
        &gtk::Separator::builder()
            .margin_bottom(3)
            .margin_top(3)
            .build(),
    );

    let all_buttons = Rc::new(RefCell::new(vec![]));
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
        &all_buttons,
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
        &all_buttons,
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
        &all_buttons,
    );

    box_.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    let tooltip = pgettext("controls", "Enable unit {}");
    create_menu_button(
        &box_,
        //Button label
        &pgettext("controls", "Enable"),
        &tooltip,
        "empty-icon",
        unit,
        MenuAction::Enable,
        unit_list_panel,
        &all_buttons,
    );

    let tooltip = pgettext("controls", "Disable unit {}");
    create_menu_button(
        &box_,
        //Button label
        &pgettext("controls", "Disable"),
        &tooltip,
        "empty-icon",
        unit,
        MenuAction::Disable,
        unit_list_panel,
        &all_buttons,
    );

    box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    /*
       create_menu_button(
           &box_,
           //Button label
           &pgettext("controls", "Mask"),
           &tooltip,
           "venetian-mask-symbolic",
           unit,
           MenuAction::Mask,
           unit_list_panel,
           &all_buttons,
       );

       create_menu_button(
           &box_,
           //Button label
           &pgettext("controls", "UnMask"),
           &tooltip,
           "venetian-unmask-symbolic",
           unit,
           MenuAction::UnMask,
           unit_list_panel,
           &all_buttons,
       );
    */
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
    all_buttons: &Rc<RefCell<Vec<(MenuAction, gtk::Button)>>>,
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

    set_button_style(unit, action, &button);
    //println!("PUSH {action:?} {:?}", button.label());
    all_buttons.borrow_mut().push((action, button.clone()));
    let unit = unit.clone();
    let all_buttons2 = all_buttons.clone();
    button.connect_clicked(move |button| {
        let inter_message = match action {
            MenuAction::Start => {
                let all_buttons = all_buttons2.clone();
                let unit = unit.clone();
                InterPanelMessage::StartUnit(
                    button,
                    &unit.clone(),
                    Rc::new(Box::new(move || {
                        set_all_button_style(&unit, &all_buttons.borrow())
                    })),
                )
            }
            MenuAction::Stop => {
                let all_buttons = all_buttons2.clone();
                let unit = unit.clone();
                InterPanelMessage::StopUnit(
                    button,
                    &unit.clone(),
                    Rc::new(Box::new(move || {
                        set_all_button_style(&unit, &all_buttons.borrow())
                    })),
                )
            }
            MenuAction::Restart => {
                let all_buttons = all_buttons2.clone();
                let unit = unit.clone();
                InterPanelMessage::ReStartUnit(
                    button,
                    &unit.clone(),
                    Rc::new(Box::new(move || {
                        set_all_button_style(&unit, &all_buttons.borrow())
                    })),
                )
            }
            MenuAction::Enable => {
                let all_buttons = all_buttons2.clone();
                let unit = unit.clone();
                InterPanelMessage::EnableUnit(
                    &unit.clone(),
                    Rc::new(Box::new(move || {
                        set_all_button_style(&unit, &all_buttons.borrow())
                    })),
                )
            }
            MenuAction::Disable => {
                let all_buttons = all_buttons2.clone();
                let unit = unit.clone();
                InterPanelMessage::DisableUnit(
                    &unit.clone(),
                    Rc::new(Box::new(move || {
                        set_all_button_style(&unit, &all_buttons.borrow())
                    })),
                )
            }
        };
        unit_list_panel.button_action(&inter_message);
    });
    box_.append(&button);
}

fn set_all_button_style(unit: &UnitInfo, all_buttons: &Vec<(MenuAction, gtk::Button)>) {
    for (action, but) in all_buttons {
        set_button_style(unit, *action, but);
    }
}

fn set_button_style(unit: &UnitInfo, action: MenuAction, button: &gtk::Button) {
    match action {
        MenuAction::Start => {
            if unit.active_state().is_inactive() {
                button.remove_css_class(FLAT);
                button.add_css_class(SUGGESTED_ACTION);
            } else {
                button.add_css_class(FLAT);
                button.remove_css_class(SUGGESTED_ACTION);
            }
        }
        MenuAction::Stop => {
            if !unit.active_state().is_inactive() {
                button.add_css_class(DESTRUCTIVE_ACTION);
            } else {
                button.remove_css_class(DESTRUCTIVE_ACTION);
            }
        }

        MenuAction::Enable => {
            let m = !matches!(
                unit.enable_status(),
                EnablementStatus::Enabled | EnablementStatus::Masked
            );

            button.set_sensitive(m);
        }
        MenuAction::Disable => {
            let m = !matches!(
                unit.enable_status(),
                EnablementStatus::Disabled | EnablementStatus::Masked
            );

            button.set_sensitive(m);
        }
        _ => {}
    };
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
