mod dropdown;
mod imp;
mod substate;
pub mod unit_prop_filter;

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};

use crate::{
    systemd::{
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus, LoadState, Preset, UnitDBusLevel},
    },
    widget::unit_list::filter::unit_prop_filter::{FilterElementAssessor, FilterTextAssessor},
};

use super::UnitListPanel;

// ANCHOR: mod
glib::wrapper! {
    pub struct UnitListFilterWindow(ObjectSubclass<imp::UnitListFilterWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitListFilterWindow {
    pub fn new(selected_filter: Option<String>, unit_list_panel: &UnitListPanel) -> Self {
        let selected_filter = if selected_filter.is_none() {
            Some("unit".to_owned())
        } else {
            selected_filter
        };

        let obj: UnitListFilterWindow = glib::Object::builder()
            .property("selected", selected_filter)
            .build();

        let _ = obj.imp().unit_list_panel.set(unit_list_panel.clone());

        obj
    }

    pub fn construct_filter_dialog(&self) {
        self.imp().get_filter()
    }
}

pub fn filter_unit_name(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool {
    let name = unit.display_name();
    property_assessor.filter_unit_value(&name)
}

pub fn filter_bus_level(
    property_assessor: &FilterElementAssessor<UnitDBusLevel>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.dbus_level())
}

pub fn filter_unit_type(
    property_assessor: &FilterElementAssessor<String>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.unit_type())
}

pub fn filter_preset(property_assessor: &FilterElementAssessor<Preset>, unit: &UnitInfo) -> bool {
    property_assessor.filter_unit_value(&unit.preset())
}

pub fn filter_enable_status(
    property_assessor: &FilterElementAssessor<EnablementStatus>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.enable_status())
}

pub fn filter_load_state(
    property_assessor: &FilterElementAssessor<LoadState>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.load_state())
}

pub fn filter_active_state(
    property_assessor: &FilterElementAssessor<ActiveState>,
    unit: &UnitInfo,
) -> bool {
    let active_state = unit.active_state();
    property_assessor.filter_unit_value(&active_state)
}

pub fn filter_sub_state(
    property_assessor: &FilterElementAssessor<String>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.sub_state())
}

pub fn filter_unit_description(property_assessor: &FilterTextAssessor, unit: &UnitInfo) -> bool {
    property_assessor.filter_unit_value(&unit.description())
}
