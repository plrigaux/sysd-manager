use log::{debug, info, warn};

use gtk::{Popover, gdk::Rectangle, prelude::*};

use crate::widget::unit_list::imp::rowdata::UnitBinding;

pub fn setup_popup_menu(units_browser: &gtk::ColumnView, filtered_list: &gtk::FilterListModel) {
    let gesture = gtk::GestureClick::builder()
        .button(gtk::gdk::BUTTON_SECONDARY)
        .build();
    {
        let units_browser_clone: gtk::ColumnView = units_browser.clone();
        let filtered_list: gtk::FilterListModel = filtered_list.clone();

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

            if let Some(object) = filtered_list.item(line_id as u32) {
                let unit_binding = object.downcast::<UnitBinding>().expect("Ok");
                info!("Pointing on Unit {}", unit_binding.primary());
                menu_show(&units_browser_clone, x as i32, y as i32);
            } else {
                warn!("Somthing wrong");
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

fn menu_show(units_browser: &gtk::ColumnView, x: i32, y: i32) {
    let menu = Popover::builder()
        .pointing_to(&Rectangle::new(x, y, 1, 1))
        .autohide(true)
        .position(gtk::PositionType::Right)
        .build();

    let box_ = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    menu.set_parent(units_browser);
    menu.set_child(Some(&box_));

    let button = gtk::Button::builder().label("Start").build();
    box_.append(&button);

    let button = gtk::Button::builder().label("stop").build();
    box_.append(&button);
    let button = gtk::Button::builder().label("ReStart").build();
    box_.append(&button);

    menu.popup();
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
