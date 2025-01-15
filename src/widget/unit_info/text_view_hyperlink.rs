use std::{cell::RefCell, rc::Rc};

use gtk::{
    gdk, gio,
    glib::{self, Value},
    pango,
    prelude::*,
};
use log::{info, warn};

use crate::{systemd, widget::app_window::AppWindow};

use super::writer::{PROP_UNDERLINE, TAG_DATA_LINK};

pub struct LinkActivator {
    app: Option<AppWindow>,
}

impl LinkActivator {
    pub fn new(app: Option<AppWindow>) -> Self {
        LinkActivator { app }
    }

    pub fn run(&self, link: &str) {
        if link.starts_with("file://") {
            let uri = gio::File::for_uri(link);
            let launcher = gtk::FileLauncher::new(Some(&uri));
            let link = link.to_owned();
            launcher.launch(
                None::<&gtk::Window>,
                None::<&gio::Cancellable>,
                move |result| {
                    if let Err(error) = result {
                        warn!("Finished launch {} Error {:?}", link, error)
                    }
                },
            );
        } else if link.starts_with("http://")
            || link.starts_with("https://")
            || link.starts_with("man:")
        {
            let launcher = gtk::UriLauncher::new(link);
            let link = link.to_owned();
            launcher.launch(
                None::<&gtk::Window>,
                None::<&gio::Cancellable>,
                move |result| {
                    if let Err(error) = result {
                        warn!("Finished launch {} Error {:?}", link, error)
                    }
                },
            );
        } else if let Some(unit_name) = link.strip_prefix("unit://") {
            info!("open unit dependency {:?} ", unit_name);
            let unit = match systemd::fetch_unit(unit_name) {
                Ok(unit) => Some(unit),
                Err(e) => {
                    warn!("Cli unit: {:?}", e);
                    None
                }
            };

            if let Some(app_window) = &self.app {
                app_window.set_unit(unit)
            }
        } else {
            warn!("Not handle link {:?}", link)
        }
    }
}

impl Clone for LinkActivator {
    fn clone(&self) -> LinkActivator {
        LinkActivator {
            app: self.app.clone(),
        }
    }
}

pub fn build_textview_link_platform(
    text_view_original: &gtk::TextView,
    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,
    link_activator: LinkActivator,
) {
    text_view_original.set_has_tooltip(true);

    text_view_original.connect_query_tooltip(|text_view, mut x, mut y, keyboard_mode, tool_tip| {
        let s = format!("TT x {} y {} b {}", x, y, keyboard_mode);
        tool_tip.set_text(Some(&s));

        //Adjust to the scrolling
        if let Some(vadj) = text_view.vadjustment() {
            y += vadj.value() as i32;
        }

        //Adjust to the scrolling
        if let Some(hadj) = text_view.hadjustment() {
            x += hadj.value() as i32;
        }

        let Some(iter) = text_view.iter_at_location(x, y) else {
            return false;
        };

        let Some(link) = retreive_tag_link_value(iter) else {
            return false;
        };

        tool_tip.set_text(Some(&link));

        true
    });

    {
        let text_view = text_view_original.clone();
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
        text_view_original.add_controller(event_controller_key);
    }

    {
        let event_controller_motion = gtk::EventControllerMotion::new();
        let text_view = text_view_original.clone();
        let hovering_over_link_tag = hovering_over_link_tag.clone();

        event_controller_motion.connect_motion(move |_motion_ctl, x, y| {
            let (tx, ty) =
                text_view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);

            set_cursor_if_appropriate(hovering_over_link_tag.clone(), &text_view, tx, ty);
        });
        text_view_original.add_controller(event_controller_motion);
    }

    {
        let gesture_click = gtk::GestureClick::new();
        let text_view = text_view_original.clone();
        let link_activator = link_activator.clone();
        gesture_click.connect_released(move |_gesture_click, _n_press, mut x, mut y| {
            let buf = text_view.buffer();

            //Adjust to the scrolling
            if let Some(vadj) = text_view.vadjustment() {
                y += vadj.value();
            }

            //Adjust to the scrolling
            if let Some(hadj) = text_view.hadjustment() {
                x += hadj.value();
            }

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

        text_view_original.add_controller(gesture_click);
    }
}

fn set_cursor_if_appropriate(
    hovering_over_link_tag: Rc<RefCell<Option<gtk::TextTag>>>,
    text_view: &gtk::TextView,
    x: i32,
    y: i32,
) {
    let mut hovering_tag: Option<gtk::TextTag> = None;

    if let Some(iter) = text_view.iter_at_location(x, y) {
        let tags = iter.tags();
        for tag in tags.iter() {
            let val = unsafe {
                let val: Option<std::ptr::NonNull<Value>> = tag.data(TAG_DATA_LINK);
                val
            };

            if val.is_some() {
                hovering_tag = Some(tag.clone());
                break;
            }
        }
    }

    let (change, previous_not_null) = {
        let previous_tag = hovering_over_link_tag.borrow();

        //It works empiricaly
        (!previous_tag.eq(&hovering_tag), previous_tag.is_some())
    };

    if change {
        if let Some(ref hovering_tag) = hovering_tag {
            text_view.set_cursor_from_name(Some("pointer"));

            hovering_tag.set_property(PROP_UNDERLINE, pango::Underline::DoubleLine.to_value());

            reset_hyper_tag(&hovering_over_link_tag, previous_not_null);
        } else {
            text_view.set_cursor_from_name(Some("text"));

            reset_hyper_tag(&hovering_over_link_tag, previous_not_null);
        }

        hovering_over_link_tag.replace(hovering_tag);
    }
}

fn reset_hyper_tag(
    hovering_over_link_tag: &Rc<RefCell<Option<gtk::TextTag>>>,
    previous_not_null: bool,
) {
    if previous_not_null {
        let previous_tag = hovering_over_link_tag.borrow();
        previous_tag
            .as_ref()
            .unwrap()
            .set_property(PROP_UNDERLINE, pango::Underline::SingleLine.to_value())
    }
}

fn follow_if_link(text_iter: gtk::TextIter, link_activator: LinkActivator) {
    let link = retreive_tag_link_value(text_iter);

    if let Some(link) = link {
        link_activator.run(&link);
    }
}

fn retreive_tag_link_value(text_iter: gtk::TextIter) -> Option<String> {
    let tags = text_iter.tags();

    let mut link_value = None;
    for tag in tags.iter() {
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
            Ok(link) => return Some(link),
            Err(e) => warn!("Link value Error {:?}", e),
        }
    }
    None
}
