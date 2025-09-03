use std::{cell::Ref, ops::DerefMut};

use adw::subclass::prelude::ObjectSubclassIsExt;
use gio::glib::property::PropertySet;
use gtk::glib::{self};

use crate::systemd::UnitProperty;

glib::wrapper! {
    pub struct PropertiesSelectorObject(ObjectSubclass<imp::PropertiesSelectorOpjectImpl>);
}

impl PropertiesSelectorObject {
    pub fn new_interface(interface: String) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().interface.replace(interface);

        this_object
    }

    pub fn from(p: UnitProperty) -> Self {
        let this_object: Self = glib::Object::new();

        let p_imp = this_object.imp();
        p_imp.unit_property.replace(p.name);
        p_imp.signature.replace(p.signature);
        p_imp.access.replace(p.access);

        this_object
    }

    pub fn add_child(&self, child: PropertiesSelectorObject) {
        //v.as_deref_mut().push(child);
        if let Some(v) = self.imp().children.borrow_mut().deref_mut() {
            v.push(child);
        } else {
            let v = vec![child];
            self.imp().children.set(Some(v));
        }
    }

    pub fn children(&self) -> Ref<'_, Option<Vec<PropertiesSelectorObject>>> {
        self.imp().children.borrow()
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::PropertiesSelectorObject)]
    pub struct PropertiesSelectorOpjectImpl {
        #[property(get)]
        pub(super) interface: RefCell<String>,
        #[property(get)]
        pub(super) unit_property: RefCell<String>,
        #[property(get)]
        pub(super) signature: RefCell<String>,
        #[property(get)]
        pub(super) access: RefCell<String>,

        pub(super) children: RefCell<Option<Vec<super::PropertiesSelectorObject>>>,
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
