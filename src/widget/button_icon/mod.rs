use crate::gtk::{glib, subclass::prelude::*};

glib::wrapper! {
    pub struct ButtonIcon(ObjectSubclass<imp::ButtonIconImpl>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable;
}

impl ButtonIcon {
    pub fn new(label: &str, icon_name: &str) -> Self {
        let obj: ButtonIcon = glib::Object::new();
        obj.set_button_icon(icon_name);
        obj.set_button_label(label);

        obj
    }

    pub fn set_button_label(&self, label: &str) {
        self.imp().button_label.set_label(label);
    }

    pub fn set_button_icon(&self, icon_name: &str) {
        self.imp().button_icon.set_icon_name(Some(icon_name));
    }
}

mod imp {

    use std::cell::RefCell;

    use gtk::prelude::*;
    use gtk::{glib, subclass::prelude::*};

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/button_icon.ui")]
    #[properties(wrapper_type = super::ButtonIcon)]
    pub struct ButtonIconImpl {
        #[property(get, set)]
        pub(super) button_icon_name: RefCell<String>,

        #[property(get, set)]
        pub(super) button_label_text: RefCell<String>,

        #[template_child]
        pub button_icon: TemplateChild<gtk::Image>,

        #[template_child]
        pub button_label: TemplateChild<gtk::Label>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ButtonIconImpl {
        const NAME: &'static str = "ButtonIcon";
        type Type = super::ButtonIcon;
        type ParentType = gtk::Button;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ButtonIconImpl {
        fn constructed(&self) {
            self.parent_constructed();

            // Bind label to number
            // `SYNC_CREATE` ensures that the label will be immediately set
            let obj = self.obj();
            obj.bind_property::<gtk::Label>("button_label_text", self.button_label.as_ref(), "label")
                .sync_create()
                .build();

                obj.bind_property::<gtk::Image>("button_icon_name", self.button_icon.as_ref(), "icon-name")
                .sync_create()
                .build();
        }
    }
    impl WidgetImpl for ButtonIconImpl {}
    impl ButtonImpl for ButtonIconImpl {}
}
