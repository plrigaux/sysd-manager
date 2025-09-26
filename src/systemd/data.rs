use std::{cmp::Ordering, fmt::Debug};

use super::{SystemdUnitFile, UpdatedUnitInfo, enums::UnitDBusLevel};

use gtk::{
    glib::{self},
    subclass::prelude::*,
};
use serde::Deserialize;
use zvariant::{OwnedObjectPath, OwnedValue, Type, Value};

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

impl Default for UnitInfo {
    fn default() -> Self {
        UnitInfo::new()
    }
}

impl UnitInfo {
    fn new() -> Self {
        let this_object: Self = glib::Object::new();
        this_object
    }

    pub fn from_listed_unit(listed_unit: LUnit, level: UnitDBusLevel) -> Self {
        let this_object: Self = glib::Object::new();
        let imp = this_object.imp();
        imp.init_from_listed_unit(listed_unit, level);
        this_object
    }

    pub fn from_unit_file(unit_file: SystemdUnitFile) -> Self {
        let this_object: Self = glib::Object::new();
        this_object.imp().init_from_unit_file(unit_file);
        this_object
    }

    pub fn update_from_unit_info(&self, update: UpdatedUnitInfo) {
        self.imp().update_from_unit_info(update);
    }

    pub fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
        self.imp().update_from_unit_file(unit_file);
    }

    pub fn debug(&self) -> String {
        format!("{:#?}", *self.imp())
    }

    pub fn set_property_values(&self, property_value_list: Vec<Option<OwnedValue>>) {
        self.imp().set_property_values(property_value_list);
    }

    pub fn custom_property(&self, property_index: usize) -> Option<String> {
        self.imp().custom_property(property_index)
    }
}

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    use zvariant::OwnedValue;

    use crate::systemd::{
        SystemdUnitFile, UpdatedUnitInfo,
        enums::{ActiveState, EnablementStatus, LoadState, Preset, UnitDBusLevel, UnitType},
        sysdbus::{self},
    };

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitInfo)]
    pub struct UnitInfoImpl {
        #[property(get, set = Self::set_primary )]
        pub(super) primary: RefCell<String>,
        #[property(get)]
        display_name: RefCell<String>,
        #[property(get, default)]
        unit_type: RefCell<UnitType>,
        #[property(get, set)]
        pub(super) description: RefCell<String>,

        #[property(get, set, default)]
        pub(super) load_state: RefCell<LoadState>,

        #[property(get, set, builder(ActiveState::Unknown))]
        pub(super) active_state: RefCell<ActiveState>,

        #[property(get, set)]
        pub(super) sub_state: RefCell<String>,
        #[property(get)]
        pub(super) followed_unit: RefCell<String>,

        //#[property(get = Self::has_object_path, name = "pathexists", type = bool)]
        #[property(get=Self::get_unit_path, type = String)]
        pub(super) object_path: RefCell<Option<String>>,
        #[property(get, set, nullable, default = None)]
        pub(super) file_path: RefCell<Option<String>>,
        #[property(get, set, default)]
        pub(super) enable_status: RefCell<EnablementStatus>,

        #[property(get, set, default)]
        pub(super) dbus_level: RefCell<UnitDBusLevel>,

        #[property(get, set, default)]
        pub(super) preset: RefCell<Preset>,

        custom_properties: RefCell<Vec<Option<OwnedValue>>>,
        //custom_properties: Arc<RefCell<Option<HashMap<String, OwnedValue>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitInfoImpl {
        const NAME: &'static str = "UnitInfo";
        type Type = super::UnitInfo;

        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for UnitInfoImpl {}

    impl UnitInfoImpl {
        pub(super) fn init_from_listed_unit(
            &self,
            listed_unit: super::LUnit,
            dbus_level: UnitDBusLevel,
        ) {
            let active_state: ActiveState = listed_unit.active_state.as_str().into();

            self.set_primary(listed_unit.primary_unit_name);
            self.active_state.replace(active_state);

            self.description.replace(listed_unit.description);
            let load_state: LoadState = listed_unit.load_state.as_str().into();
            self.load_state.replace(load_state);
            self.sub_state.replace(listed_unit.sub_state);
            self.followed_unit.replace(listed_unit.followed_unit);
            let unit_object_path = Some(listed_unit.unit_object_path.to_string());
            self.object_path.replace(unit_object_path);
            self.dbus_level.replace(dbus_level);
        }

        pub(super) fn init_from_unit_file(&self, unit_file: SystemdUnitFile) {
            self.set_primary(unit_file.full_name);
            //self.set_active_state(ActiveState::Unknown);
            self.dbus_level.replace(unit_file.level);
            self.file_path.replace(Some(unit_file.path));
            self.enable_status.replace(unit_file.status_code);
        }

        pub(super) fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
            self.file_path.replace(Some(unit_file.path));
            self.enable_status.replace(unit_file.status_code);
        }

        pub fn set_primary(&self, primary: String) {
            let mut split_char_index = primary.len();
            for (i, c) in primary.chars().rev().enumerate() {
                if c == '.' {
                    split_char_index -= i;
                    break;
                }
            }

            let display_name = primary[..split_char_index - 1].to_owned();
            self.display_name.replace(display_name);

            let unit_type = UnitType::new(&primary[(split_char_index)..]);
            self.unit_type.replace(unit_type);

            self.primary.replace(primary);
        }

        pub fn update_from_unit_info(&self, update: UpdatedUnitInfo) {
            self.object_path.replace(Some(update.object_path));

            if let Some(description) = update.description {
                self.description.replace(description);
            }

            if let Some(sub_state) = update.sub_state {
                self.sub_state.replace(sub_state);
            }

            if let Some(active_state) = update.active_state {
                self.active_state.replace(active_state);
            }

            if let Some(unit_file_preset) = update.unit_file_preset {
                let preset: Preset = unit_file_preset.into();
                self.preset.replace(preset);
            }

            if let Some(load_state) = update.load_state {
                self.load_state.replace(load_state);
            }

            if let Some(fragment_path) = update.fragment_path {
                self.file_path.replace(Some(fragment_path));
            }

            if let Some(enablement_status) = update.enablement_status {
                self.enable_status.replace(enablement_status);
            }
        }

        fn get_unit_path(&self) -> String {
            if let Some(a) = &*self.object_path.borrow() {
                a.clone()
            } else {
                let primary = &*self.primary.borrow();
                let object_path = sysdbus::unit_dbus_path_from_name(primary);
                self.object_path.replace(Some(object_path.clone()));
                object_path
            }
        }

        pub fn set_property_values(&self, property_value_list: Vec<Option<OwnedValue>>) {
            self.custom_properties.replace(property_value_list);
        }

        pub fn custom_property(&self, property_index: usize) -> Option<String> {
            let vec = self.custom_properties.borrow();

            if property_index >= vec.len() {
                /*  warn!(
                    "Property vector request out of bound!  Index {property_index} {}",
                    vec.len()
                ); */
                None
            } else if let Some(v) = vec.get(property_index)
                && let Some(v) = v
            {
                let s = super::convert_to_string(v);
                Some(s)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct UnitProcess {
    pub path: String,
    pub pid: u32,
    pub name: String,
    pub(crate) unit_name: usize,
}

impl UnitProcess {
    pub fn unit_name(&self) -> &str {
        &self.path[self.unit_name..]
    }
}

impl Ord for UnitProcess {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp: Ordering = self.unit_name().cmp(other.unit_name());
        if self.unit_name().cmp(other.unit_name()) == Ordering::Equal {
            self.pid.cmp(&other.pid)
        } else {
            cmp
        }
    }
}

impl PartialOrd for UnitProcess {
    fn partial_cmp(&self, other: &UnitProcess) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Type, Deserialize)]
#[allow(unused)]
pub struct DisEnAbleUnitFiles {
    pub change_type: String,
    pub file_name: String,
    pub destination: String,
}

#[derive(Debug, Type, Deserialize)]
#[allow(unused)]
pub struct EnableUnitFilesReturn {
    pub carries_install_info: bool,
    pub vec: Vec<DisEnAbleUnitFiles>,
}

#[derive(Deserialize, zvariant::Type, PartialEq, Debug)]
pub struct LUnit {
    pub primary_unit_name: String,
    pub description: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub followed_unit: String,

    pub unit_object_path: OwnedObjectPath,
    ///If there is a job queued for the job unit the numeric job id, 0 otherwise
    pub numeric_job_id: u32,
    pub job_type: String,
    pub job_object_path: OwnedObjectPath,
}

pub fn convert_to_string(value: &Value) -> String {
    match value {
        Value::U8(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::I16(i) => i.to_string(),
        Value::U16(i) => i.to_string(),
        Value::I32(i) => i.to_string(),
        Value::U32(i) => i.to_string(),
        Value::I64(i) => i.to_string(),
        Value::U64(i) => i.to_string(),
        Value::F64(i) => i.to_string(),
        Value::Str(s) => s.to_string(),
        Value::Signature(s) => s.to_string(),
        Value::ObjectPath(op) => op.to_string(),
        Value::Value(v) => v.to_string(),
        Value::Array(a) => {
            if a.is_empty() {
                "[]".to_owned()
            } else {
                let mut d_str = String::from("[ ");

                let mut it = a.iter().peekable();
                while let Some(mi) = it.next() {
                    let sub_value = convert_to_string(mi);

                    d_str.push_str(&sub_value);
                    if it.peek().is_some() {
                        d_str.push_str(", ");
                    }
                }

                d_str.push_str(" ]");
                d_str
            }
        }
        Value::Dict(d) => {
            let mut d_str = String::from("{ ");

            for (mik, miv) in d.iter() {
                d_str.push_str(&convert_to_string(mik));
                d_str.push_str(" : ");
                d_str.push_str(&convert_to_string(miv));
            }
            d_str.push_str(" }");
            d_str
        }
        Value::Structure(stc) => {
            let mut d_str = String::from("{ ");

            let mut it = stc.fields().iter().peekable();

            while let Some(mi) = it.next() {
                let sub_value = convert_to_string(mi);

                d_str.push_str(&sub_value);
                if it.peek().is_some() {
                    d_str.push_str(", ");
                }
            }

            d_str.push_str(" }");
            d_str
        }
        Value::Fd(fd) => fd.to_string(),
        //Value::Maybe(maybe) => (maybe.to_string(), false),
    }
}
