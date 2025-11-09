mod dropdown;
mod imp;
mod substate;
pub mod unit_prop_filter;

use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{
    glib::{self},
    prelude::ObjectExt,
};
use log::warn;
use zvariant::OwnedValue;

use crate::{
    systemd::{
        data::UnitInfo,
        enums::{ActiveState, EnablementStatus, LoadState, Preset, UnitDBusLevel, UnitType},
    },
    widget::unit_list::{
        COL_ID_UNIT,
        filter::unit_prop_filter::{
            FilterBoolAssessor, FilterElementAssessor, FilterNumAssessor, FilterTextAssessor,
        },
    },
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
            Some(COL_ID_UNIT.to_owned())
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

pub fn filter_unit_name(
    property_assessor: &FilterTextAssessor,
    unit: &UnitInfo,
    _key: glib::Quark,
) -> bool {
    let name = unit.display_name();
    (property_assessor.filter_unit_value_func)(property_assessor, Some(&name))
}

pub fn filter_bus_level(
    property_assessor: &FilterElementAssessor<UnitDBusLevel>,
    unit: &UnitInfo,
) -> bool {
    property_assessor.filter_unit_value(&unit.dbus_level())
}

pub fn filter_unit_type(
    property_assessor: &FilterElementAssessor<UnitType>,
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

pub fn filter_unit_description(
    property_assessor: &FilterTextAssessor,
    unit: &UnitInfo,
    _key: glib::Quark,
) -> bool {
    (property_assessor.filter_unit_value_func)(property_assessor, Some(&unit.description()))
}

pub fn custom_num<T>(
    property_assessor: &FilterNumAssessor<T>,
    unit: &UnitInfo,
    key: glib::Quark,
) -> bool
where
    T: std::fmt::Debug + std::default::Default + for<'a> TryFrom<&'a zvariant::Value<'a>>,
    for<'a> zvariant::Error:
        std::convert::From<<T as std::convert::TryFrom<&'a zvariant::Value<'a>>>::Error>,
{
    let value = unsafe { unit.qdata::<OwnedValue>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .map(|value| {
            value
                .downcast_ref::<T>()
                .inspect_err(|e| warn!("wrong type mapping {e:?}"))
                .unwrap_or_default()
        });
    (property_assessor.filter_unit_value_func)(property_assessor, value)
}

pub fn custom_str(
    property_assessor: &FilterTextAssessor,
    unit: &UnitInfo,
    key: glib::Quark,
) -> bool {
    let value = unsafe { unit.qdata::<OwnedValue>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .map(|value| {
            value
                .downcast_ref::<String>()
                .inspect_err(|e| warn!("wrong type mapping {e:?}"))
                .unwrap_or_default()
        });
    (property_assessor.filter_unit_value_func)(property_assessor, value.as_deref())
}

pub fn custom_bool(
    property_assessor: &FilterBoolAssessor,
    unit: &UnitInfo,
    key: glib::Quark,
) -> bool {
    let value = unsafe { unit.qdata::<OwnedValue>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .map(|value| {
            value
                .downcast_ref::<bool>()
                .inspect_err(|e| warn!("wrong type mapping {e:?}"))
                .unwrap_or_default()
        });
    (property_assessor.filter_unit_value_func)(property_assessor, value)
}
