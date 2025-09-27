use gtk::{
    ListItem,
    glib::{self},
    subclass::prelude::*,
};

use crate::widget::unit_properties_selector::{
    data::UnitPropertySelection, unit_properties_selection::UnitPropertiesSelection,
};

glib::wrapper! {
    pub struct UnitPropertiesSelectionRow(ObjectSubclass<imp::UnitPropertiesSelectionRowImp>)
    @extends gtk::Grid, gtk::Widget,
    @implements gtk::Actionable, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitPropertiesSelectionRow {
    pub fn new(prop_selection: UnitPropertiesSelection) -> Self {
        let obj: UnitPropertiesSelectionRow = glib::Object::new();

        obj.imp().prop_selection.replace(Some(prop_selection));

        obj
    }

    pub fn set_data_selection(&self, prop_seletion: &UnitPropertySelection, list_item: &ListItem) {
        self.imp().set_data_selection(prop_seletion, list_item)
    }

    pub fn unbind(&self) {
        self.imp().unbind()
    }
}

impl Default for UnitPropertiesSelectionRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use std::cell::RefCell;

    use crate::widget::unit_properties_selector::{
        data::UnitPropertySelection, unit_properties_selection::UnitPropertiesSelection,
    };

    use super::UnitPropertiesSelectionRow;
    use gio::glib::Binding;
    use gtk::{
        glib::{self},
        prelude::*,
        subclass::prelude::*,
    };
    use log::warn;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/unit_properties_selection_row.ui")]
    pub struct UnitPropertiesSelectionRowImp {
        #[template_child]
        interface: TemplateChild<gtk::Label>,
        #[template_child]
        property_label: TemplateChild<gtk::Label>,
        #[template_child]
        hidden_ckeck: TemplateChild<gtk::CheckButton>,

        pub prop_selection: RefCell<Option<UnitPropertiesSelection>>,

        list_item: RefCell<Option<gtk::ListItem>>,

        bind: RefCell<Option<Binding>>,
    }

    #[gtk::template_callbacks]
    impl UnitPropertiesSelectionRowImp {
        pub(super) fn set_data_selection(
            &self,
            prop_seletion: &UnitPropertySelection,
            list_item: &gtk::ListItem,
        ) {
            self.list_item.replace(Some(list_item.clone()));

            let interface = prop_seletion.interface();

            if let Some(token) = interface.split('.').next_back() {
                self.interface.set_label(token);
            } else {
                self.interface.set_label(&interface);
            }

            self.property_label
                .set_label(&prop_seletion.unit_property());

            self.hidden_ckeck.set_active(prop_seletion.visible());
            let bind = self
                .hidden_ckeck
                .bind_property("active", prop_seletion, "visible")
                .build();

            let old_bind = self.bind.replace(Some(bind));

            if let Some(old_bind) = old_bind {
                old_bind.unbind();
            }
        }

        pub(super) fn unbind(&self) {
            if let Some(old_bind) = self.bind.borrow().as_ref() {
                old_bind.unbind();
            }

            self.list_item.replace(None);
        }
        #[template_callback]
        fn move_up_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            let pos = list_item.position();
            if pos == 0 {
                return;
            }

            if let Some(list_store) = self
                .prop_selection
                .borrow()
                .as_ref()
                .and_then(|prop_selection| prop_selection.list_store())
                && let Some(ref prop_seletion) = list_store.item(pos)
            {
                list_store.remove(pos);
                list_store.insert(pos - 1, prop_seletion);
            } else {
                warn!("list_store of data None");
            };
        }

        #[template_callback]
        fn move_down_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            if let Some(prop_selection) = self.prop_selection.borrow().as_ref() {
                let pos = list_item.position();
                prop_selection.move_down(pos);
            }
        }

        #[template_callback]
        fn delete_clicked(&self, _b: &gtk::Button) {
            let Some(ref list_item) = *self.list_item.borrow() else {
                return;
            };

            if let Some(prop_selection) = self.prop_selection.borrow().as_ref() {
                let pos = list_item.position();
                prop_selection.delete(pos);
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitPropertiesSelectionRowImp {
        const NAME: &'static str = "UnitPropertiesSelectorRow";
        type Type = UnitPropertiesSelectionRow;
        type ParentType = gtk::Grid;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnitPropertiesSelectionRowImp {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for UnitPropertiesSelectionRowImp {}
    impl GridImpl for UnitPropertiesSelectionRowImp {}
}
