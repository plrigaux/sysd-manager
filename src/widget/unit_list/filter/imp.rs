use std::cell::RefCell;

use adw::{prelude::*, subclass::window::AdwWindowImpl};

use gtk::{
    glib::{self},
    subclass::{
        prelude::*,
        widget::{
            CompositeTemplateCallbacksClass, CompositeTemplateClass,
            CompositeTemplateInitializingExt, WidgetImpl,
        },
    },
};
use strum::IntoEnumIterator;

use crate::{
    systemd::enums::{ActiveState, EnablementStatus, UnitType},
    widget::preferences::data::UNIT_LIST_COLUMNS,
};

use super::UnitListFilterWindow;

#[derive(Default, gtk::CompositeTemplate, glib::Properties)]
#[template(resource = "/io/github/plrigaux/sysd-manager/unit_list_filter.ui")]
#[properties(wrapper_type = super::UnitListFilterWindow)]
pub struct UnitListFilterWindowImp {
    #[template_child]
    filter_stack: TemplateChild<adw::ViewStack>,

    #[template_child]
    filter_navigation_container: TemplateChild<gtk::Box>,

    #[property(get, set, nullable, default = None)]
    selected: RefCell<Option<String>>,
}

#[gtk::template_callbacks]
impl UnitListFilterWindowImp {
    /*     #[template_callback]
    fn notify_visible_child_cb(&self, _stack: Param) {
        println!("notify_visible_child_cb");
    } */
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for UnitListFilterWindowImp {
    const NAME: &'static str = "UNIT_LIST_FILTER";
    type Type = UnitListFilterWindow;
    type ParentType = adw::Window;

    fn class_init(klass: &mut Self::Class) {
        // The layout manager determines how child widgets are laid out.
        klass.bind_template();
        klass.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for UnitListFilterWindowImp {
    fn constructed(&self) {
        self.parent_constructed();

        for (name, key, _) in UNIT_LIST_COLUMNS {
            let widget: gtk::Widget = match key {
                "type" => build_type_filter().into(),
                "state" => build_enablement_filter().into(),
                "active" => build_active_state_filter().into(),

                _ => gtk::Label::new(Some(name)).into(),
            };

            let _stack_page = self.filter_stack.add_titled(&widget, Some(key), name);

            let button_content = adw::ButtonContent::builder()
                .icon_name("empty-icon")
                .label(name)
                .halign(gtk::Align::Start)
                .css_classes(["nav"])
                .build();
            let button = gtk::Button::builder()
                .child(&button_content)
                .css_classes(["flat"])
                .build();

            {
                let filter_stack = self.filter_stack.clone();
                button.connect_clicked(move |_| {
                    filter_stack.set_visible_child_name(key);
                });
            }
            self.filter_navigation_container.append(&button);
        }

        self.obj()
            .bind_property::<adw::ViewStack>(
                "selected",
                self.filter_stack.as_ref(),
                "visible-child-name",
            )
            .bidirectional()
            .build();
    }
}

impl WidgetImpl for UnitListFilterWindowImp {}
impl WindowImpl for UnitListFilterWindowImp {}
impl AdwWindowImpl for UnitListFilterWindowImp {}

fn build_type_filter() -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 5);
    for unit_type in UnitType::iter().filter(|x| !matches!(*x, UnitType::Unknown(_))) {
        let check = gtk::CheckButton::builder()
            .label(unit_type.to_str())
            //.action_target(&unit_type.to_str().to_variant())
            .build();

        check.connect_toggled(move |check_button| {
            println!("t {} {:?}", check_button.is_active(), unit_type.to_str());
        });

        container.append(&check);
    }

    build_controls(&container);

    container
}

fn build_controls(container: &gtk::Box) {
    let controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(["linked"])
        .build();

    let clear_button = gtk::Button::builder().label("Clear").build();
    {
        let container = container.clone();
        clear_button.connect_clicked(move |_| {
            let mut some_widget = container.first_child();
            while let Some(w) = some_widget.as_ref() {
                let maybe_checkbutton = w.downcast_ref::<gtk::CheckButton>();
                if let Some(check) = maybe_checkbutton {
                    check.set_active(false);
                }

                some_widget = w.next_sibling();
            }
        });
    }

    let inv_button = gtk::Button::builder().label("Inv").build();
    {
        let container = container.clone();
        inv_button.connect_clicked(move |_| {
            let mut some_widget = container.first_child();

            while let Some(w) = some_widget.as_ref() {
                let maybe_checkbutton = w.downcast_ref::<gtk::CheckButton>();
                if let Some(check) = maybe_checkbutton {
                    check.set_active(!check.is_active());
                }

                some_widget = w.next_sibling();
            }
        });
    }
    controls.append(&clear_button);
    controls.append(&inv_button);

    container.append(&controls);
}

fn build_enablement_filter() -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 5);

    for status in EnablementStatus::iter().filter(|x| match *x {
        EnablementStatus::Unknown => false,
        //EnablementStatus::Unasigned => false,
        _ => true,
    }) {
        let check = gtk::CheckButton::with_label(status.as_str());
        container.append(&check);
    }
    build_controls(&container);
    container
}

fn build_active_state_filter() -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 5);

    for status in ActiveState::iter() {
        let check = gtk::CheckButton::with_label(status.as_str());
        container.append(&check);
    }
    build_controls(&container);
    container
}
