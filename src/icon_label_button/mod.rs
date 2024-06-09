use glib::Object;
use gtk::{
    glib::{self, property::PropertyGet},
    subclass::prelude::*,
};
use log::error;

glib::wrapper! {
    pub struct IconLabelButton(ObjectSubclass<imp::IconLabelButton>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;


}
// ANCHOR_END: mod

impl IconLabelButton {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_label_text(&self, label_text: &str) {
        let ref_label = self.imp().label.borrow();

        match ref_label.as_ref() {
            Some(l) => l.set_label(label_text),
            None => error!("No label"),
        };
    }

    pub fn set_icon_name(icon_name: &str) {}
}

impl Default for IconLabelButton {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    // ANCHOR: imp
    // Object holding the state
    #[derive(Default)]
    pub struct IconLabelButton {
        //child: RefCell<Option<gtk::Widget>>,
        box_container: RefCell<Option<gtk::Box>>,
        pub label: RefCell<Option<gtk::Label>>,
        icon: RefCell<Option<gtk::Image>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for IconLabelButton {
        const NAME: &'static str = "IconLabelButton";
        type Type = super::IconLabelButton;
        type ParentType = gtk::Button;
    }
    // ANCHOR_END: imp

    // Trait shared by all GObjects
    impl ObjectImpl for IconLabelButton {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let label = "Hello world!";
            let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
            let label1 = gtk::Label::new(Some(label));
            let icon1 = gtk::Image::from_icon_name("phone-symbolic");

            box_container.append(&icon1);
            box_container.append(&label1);

            // Create the child label.

            //let child = gtk::Label::new(Some(label));
            box_container.set_parent(&*obj);
            *self.box_container.borrow_mut() = Some(box_container);
            *self.label.borrow_mut() = Some(label1);

            // Make it look like a GTK button with a label (as opposed to an icon).
            obj.add_css_class("text-button");

            // Tell accessibility tools the button has a label.
            obj.update_property(&[gtk::accessible::Property::Label(label)]);

            // Connect a gesture to handle clicks.
            let gesture = gtk::GestureClick::new();
            gesture.connect_released(|gesture, _, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                println!("Button pressed!");
            });
            obj.add_controller(gesture);
        }

        fn dispose(&self) {
            // Child widgets need to be manually unparented in `dispose()`.
            if let Some(box_container) = self.box_container.borrow_mut().take() {
                box_container.unparent();
            }
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for IconLabelButton {}

    // Trait shared by all buttons
    impl ButtonImpl for IconLabelButton {}
}
