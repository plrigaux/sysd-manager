use std::{cell::Cell, rc::Rc};

use gtk::{
    gdk,
    glib::{self, Value},
    prelude::*,
};
use log::warn;

use crate::widget::app_window::AppWindow;

use super::writer::TAG_DATA_LINK;

pub struct LinkActivator {
    f: fn(&str, &Option<AppWindow>),
    app: Option<AppWindow>,
}

impl LinkActivator {
    pub fn new(f: fn(&str, &Option<AppWindow>), app: Option<AppWindow>) -> Self {
        LinkActivator { f, app }
    }

    pub fn run(&self, s: &str) {
        (self.f)(s, &self.app)
    }
}

impl Clone for LinkActivator {
    fn clone(&self) -> LinkActivator {
        LinkActivator {
            f: self.f,
            app: self.app.clone(),
        }
    }
}

pub fn build_textview_link_platform(
    text_view_or: &gtk::TextView,
    hovering_over_link: Rc<Cell<bool>>,
    link_activator: LinkActivator,
) {
    {
        let text_view = text_view_or.clone();
        let event_controller_key = gtk::EventControllerKey::new();
        let link_activator = link_activator.clone();
        event_controller_key.connect_key_pressed(
            move |_event_controller_key, keyval: gdk::Key, _keycode, _modifiers| {
                match keyval {
                    gdk::Key::Return | gdk::Key::KP_Enter => {
                        let buffer = text_view.buffer();
                        let mark = buffer.get_insert();
                        let iter = buffer.iter_at_mark(&mark);

                        follow_if_link(iter, link_activator.clone());
                    }
                    _ => {}
                }
                glib::Propagation::Proceed
            },
        );
        text_view_or.add_controller(event_controller_key);
    }

    {
        let event_controller_motion = gtk::EventControllerMotion::new();
        let text_view = text_view_or.clone();
        let hovering_over_link = hovering_over_link.clone();

        event_controller_motion.connect_motion(move |_motion_ctl, x, y| {
            let (tx, ty) =
                text_view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);

            set_cursor_if_appropriate(hovering_over_link.clone(), &text_view, tx, ty);
        });
        text_view_or.add_controller(event_controller_motion);
    }

    {
        let gesture_click = gtk::GestureClick::new();
        let text_view = text_view_or.clone();
        let link_activator = link_activator.clone();
        gesture_click.connect_released(move |_gesture_click, _n_press, x, y| {
            let buf = text_view.buffer();

            // we shouldn't follow a link if the user has selected something
            if let Some((start, end)) = buf.selection_bounds() {
                if start.offset() != end.offset() {
                    return;
                }
            }

            let Some(iter) = text_view.iter_at_location(x as i32, y as i32) else {
                return;
            };

            follow_if_link(iter, link_activator.clone());
        });

        text_view_or.add_controller(gesture_click);
    }
}

fn set_cursor_if_appropriate(
    hovering_over_link: Rc<Cell<bool>>,
    text_view: &gtk::TextView,
    x: i32,
    y: i32,
) {
    let mut hovering = false;
    if let Some(iter) = text_view.iter_at_location(x, y) {
        let tags = iter.tags();

        for tag in tags.iter() {
            let val = unsafe {
                let val: Option<std::ptr::NonNull<Value>> = tag.data(TAG_DATA_LINK);
                val
            };

            if val.is_some() {
                hovering = true;
                break;
            }
        }
    }

    if hovering_over_link.get() != hovering {
        hovering_over_link.set(hovering);
        if hovering {
            text_view.set_cursor_from_name(Some("pointer"));
        } else {
            text_view.set_cursor_from_name(Some("text"));
        }
    }
}

fn follow_if_link(text_iter: gtk::TextIter, link_activator: LinkActivator) {
    let tags = text_iter.tags();

    let mut link_value = None;

    for tag in tags.iter() {
        //info!("TAG {:?} {:?}", tag, tag.name());

        link_value = unsafe {
            let val: Option<std::ptr::NonNull<Value>> = tag.data(TAG_DATA_LINK);
            val.map(|link_value_nonull| link_value_nonull.as_ref())
        };

        if link_value.is_some() {
            break;
        }
    }

    if let Some(link_value) = link_value {
        match link_value.get::<String>() {
            Ok(file_link) => {
                link_activator.run(&file_link);
            }
            Err(e) => warn!("Link value Error {:?}", e),
        }
    }
}
