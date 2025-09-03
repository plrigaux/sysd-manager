use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

glib::wrapper! {
    pub struct PropertiesSelectorObject(ObjectSubclass<imp::PropertiesSelectorOpjectImpl>);
}

impl Default for PropertiesSelectorObject {
    fn default() -> Self {
        PropertiesSelectorObject::new()
    }
}

impl PropertiesSelectorObject {
    fn new() -> Self {
        let this_object: Self = glib::Object::new();
        this_object
    }

    pub fn ps(p: &String, s: &String) -> Self {
        let this_object: Self = glib::Object::new();

        this_object.set_unit_property(p.as_str());
        this_object.set_signature(s.as_str());

        this_object
    }

    pub fn add_child(&self, child: PropertiesSelectorObject) {
        self.imp().children.borrow_mut().push(child);
    }

    pub fn children(&self) -> std::cell::Ref<'_, Vec<PropertiesSelectorObject>> {
        self.imp().children.borrow()
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::PropertiesSelectorObject)]
    pub struct PropertiesSelectorOpjectImpl {
        #[property(get, set)]
        pub interface: RefCell<Option<String>>,
        #[property(get, set)]
        pub(super) unit_property: RefCell<Option<String>>,
        #[property(get, set)]
        pub(super) signature: RefCell<Option<String>>,

        pub(super) children: RefCell<Vec<super::PropertiesSelectorObject>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertiesSelectorOpjectImpl {
        const NAME: &'static str = "PROPERTIES_SELECTOR_OBJECT";
        type Type = super::PropertiesSelectorObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PropertiesSelectorOpjectImpl {}
    impl PropertiesSelectorOpjectImpl {}
}
