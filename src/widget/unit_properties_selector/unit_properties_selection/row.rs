use gtk::{
    ListItem,
    glib::{self},
    subclass::prelude::*,
};

use crate::widget::unit_properties_selector::unit_properties_selection::{
    UnitPropertiesSelection, data::UnitPropertySelection,
};
glib::wrapper! {
    pub struct UnitPropertiesSelectionRow(ObjectSubclass<imp::UnitPropertiesSelectionRowImp>)
    @extends gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Actionable, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl UnitPropertiesSelectionRow {
    pub fn new(prop_selection: UnitPropertiesSelection) -> Self {
        let obj: UnitPropertiesSelectionRow = glib::Object::new();

        obj.imp()
            .prop_selection
            .set(prop_selection)
            .expect("Only Once");
        obj
    }

    pub fn set_data_selection(&self, prop_seletion: &UnitPropertySelection, list_item: &ListItem) {
        self.imp().set_data_selection(prop_seletion, list_item)
    }
}

impl Default for UnitPropertiesSelectionRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

mod imp {
    use std::cell::{OnceCell, RefCell};

    use crate::widget::unit_properties_selector::unit_properties_selection::{
        UnitPropertiesSelection, data::UnitPropertySelection,
    };

    use super::UnitPropertiesSelectionRow;
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

        pub(super) prop_selection: OnceCell<UnitPropertiesSelection>,

        list_item: RefCell<Option<gtk::ListItem>>,
    }

    #[gtk::template_callbacks]
    impl UnitPropertiesSelectionRowImp {
        pub(super) fn set_data_selection(
            &self,
            prop_seletion: &UnitPropertySelection,
            list_item: &gtk::ListItem,
        ) {
            self.list_item.replace(Some(list_item.clone()));

            self.interface.set_label(&prop_seletion.interface());
            self.property_label
                .set_label(&prop_seletion.unit_property());
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
                .get()
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

            let Some(list_store) = self
                .prop_selection
                .get()
                .and_then(|prop_selection| prop_selection.list_store())
            else {
                return;
            };

            let pos = list_item.position();

            if pos + 1 >= list_store.n_items() {
                return;
            }

            if let Some(ref prop_seletion) = list_store.item(pos) {
                list_store.remove(pos);
                list_store.insert(pos + 1, prop_seletion);
            } else {
                warn!("list_store of data None");
            };
        }

        #[template_callback]
        fn delete_clicked(&self, _b: &gtk::Button) {
            if let Some(ref list_item) = *self.list_item.borrow()
                && let Some(list_store) = self
                    .prop_selection
                    .get()
                    .and_then(|prop_selection| prop_selection.list_store())
            {
                let pos = list_item.position();
                list_store.remove(pos);
            } else {
                warn!("list_store of data None");
            };
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for UnitPropertiesSelectionRowImp {
        const NAME: &'static str = "UnitPropertiesSelectorRow";
        type Type = UnitPropertiesSelectionRow;
        type ParentType = gtk::ListBoxRow;

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
    impl ListBoxRowImpl for UnitPropertiesSelectionRowImp {}
}
