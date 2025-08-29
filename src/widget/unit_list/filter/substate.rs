use std::rc::Weak;
use std::{cell::RefCell, rc::Rc};

use gettextrs::pgettext;
use gio::glib::WeakRef;
use log::warn;

use crate::consts::FLAT;
use crate::gtk::prelude::*;

use crate::gtk::glib::clone::Downgrade;
use crate::widget::unit_list::filter::imp::FilterWidget;
use crate::widget::unit_list::filter::unit_prop_filter::{FilterElement, UnitPropertyFilter};
use crate::widget::unit_list::filter::{
    dropdown::SubState,
    imp::{contain_entry, create_content_box},
};
pub fn sub_state_filter(
    filter_container_: &Rc<RefCell<Box<dyn UnitPropertyFilter>>>,
) -> (gtk::Box, Vec<FilterWidget>) {
    let container = create_content_box();

    let wrapbox = adw::WrapBox::builder()
        .line_spacing(5)
        .child_spacing(5)
        .build();

    container.append(&wrapbox);

    let (merge_box, entry) = contain_entry();

    let wrapper = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .build();

    let label = pgettext("filter", "Add");
    let tooltip_text = pgettext("filter", "Add filter on sub state");
    let add_button = gtk::Button::builder()
        .label(label)
        .tooltip_text(tooltip_text)
        .build();

    wrapper.append(&merge_box);
    wrapper.append(&add_button);

    container.append(&wrapper);

    let drop_down = super::dropdown::drop_down();

    container.append(&drop_down);

    //let filter_container = filter_container.clone();
    let wrapbox_weak = gtk::prelude::ObjectExt::downgrade(&wrapbox);
    let filter_container_weak = filter_container_.downgrade();
    {
        let filter_container = filter_container_.borrow();

        let filter_element = filter_container
            .as_any()
            .downcast_ref::<FilterElement<String>>()
            .expect("downcast_ref to FilterElement");

        for filter_word in filter_element.elements() {
            add_filter_tag(
                filter_word,
                &wrapbox_weak,
                filter_container_weak.clone(),
                false,
            );
        }
    }

    {
        let filter_container_weak = filter_container_weak.clone();
        let container_weak = wrapbox_weak.clone();
        add_button.connect_clicked(move |_but| {
            // let (merge_box, _entry) = contain_entry();

            let word = entry.text();
            add_filter_tag(
                word.as_str(),
                &container_weak,
                filter_container_weak.clone(),
                true,
            );
            entry.set_text("");

            if let Some(filter_container) = filter_container_weak.upgrade() {
                let mut binding = filter_container.as_ref().borrow_mut();

                let filter_text = binding
                    .as_any_mut()
                    .downcast_mut::<FilterElement<String>>()
                    .expect("downcast_mut to FilterElement");

                filter_text.set_filter_elem(word.to_string(), true);
            }
        });
    }
    {
        let wrapbox_weak = wrapbox_weak.clone();
        let filter_container_weak = filter_container_weak.clone();
        drop_down.connect_selected_notify(move |drop| {
            let selected_item = drop.selected_item();

            let sub_state = selected_item
                .and_downcast::<SubState>()
                .expect("Shall be SubState");
            // let (merge_box, _entry) = contain_entry();

            add_filter_tag(
                &sub_state.sub_state(),
                &wrapbox_weak,
                filter_container_weak.clone(),
                true,
            );

            let Some(filter_container) = filter_container_weak.upgrade() else {
                warn!("filter_container_weak None");
                return;
            };

            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_text = binding
                .as_any_mut()
                .downcast_mut::<FilterElement<String>>()
                .expect("downcast_mut to FilterElement");

            filter_text.set_filter_elem(sub_state.sub_state(), true);
        });
    }

    let controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["linked"])
        .halign(gtk::Align::Center)
        .build();

    let clear_button = gtk::Button::builder()
        .label(pgettext("filter", "Clear"))
        .tooltip_text(pgettext("filter", "Clear filter's selected items"))
        .build();
    {
        clear_button.connect_clicked(move |_| {
            if let Some(wrapbox) = wrapbox_weak.upgrade() {
                while let Some(child) = wrapbox.first_child() {
                    wrapbox.remove(&child);
                }
            }

            if let Some(filter_container) = filter_container_weak.upgrade() {
                let mut binding = filter_container.as_ref().borrow_mut();

                let filter_elem = binding
                    .as_any_mut()
                    .downcast_mut::<FilterElement<String>>()
                    .expect("downcast_ref to FilterElement");

                filter_elem.clear_n_apply_filter();
            }
        });
    }

    controls.append(&clear_button);

    container.append(&controls);

    (container, vec![FilterWidget::WrapBox(wrapbox)])
}

fn add_filter_tag(
    word: &str,
    wrapbox_weak: &WeakRef<adw::WrapBox>,
    filter_container: Weak<RefCell<Box<dyn UnitPropertyFilter>>>,
    duplicate_check: bool,
) {
    if duplicate_check && let Some(filter_container) = filter_container.upgrade() {
        let binding = filter_container.as_ref().borrow();

        let filter_elem = binding
            .as_any()
            .downcast_ref::<FilterElement<String>>()
            .expect("downcast_ref to FilterElement");

        if filter_elem.contains(&word.to_owned()) {
            return;
        }
    }

    let box_word = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(0)
        .hexpand(false)
        .css_classes(["tag"])
        .build();

    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .css_classes([FLAT, "circular"])
        .build();

    let label = gtk::Label::builder()
        .xalign(0.0)
        .ellipsize(pango::EllipsizeMode::End)
        .hexpand(true)
        .label(word)
        .build();

    box_word.append(&label);
    box_word.append(&close_button);

    if let Some(wrapbox) = wrapbox_weak.upgrade() {
        wrapbox.append(&box_word);
    }

    let wrap_box_weak = wrapbox_weak.clone();
    let box_word_weak = gtk::prelude::ObjectExt::downgrade(&box_word);
    let word = word.to_owned();

    close_button.connect_clicked(move |_b| {
        if let Some(box_word) = box_word_weak.upgrade()
            && let Some(wrapbox) = wrap_box_weak.upgrade()
        {
            wrapbox.remove(&box_word);
        }

        if let Some(filter_container) = filter_container.upgrade() {
            let mut binding = filter_container.as_ref().borrow_mut();

            let filter_elem = binding
                .as_any_mut()
                .downcast_mut::<FilterElement<String>>()
                .expect("downcast_ref to FilterElement");

            filter_elem.set_filter_elem(word.clone(), false);
        }
    });
}
