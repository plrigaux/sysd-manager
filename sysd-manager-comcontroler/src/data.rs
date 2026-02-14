use std::{cmp::Ordering, fmt::Debug};

use crate::{
    enums::{ActiveState, LoadState, Preset, UnitFileStatus, UnitType},
    sysdbus::ListedUnitFile,
};

use super::UpdatedUnitInfo;

use base::enums::UnitDBusLevel;
use glib::{self, Quark, object::ObjectExt, subclass::types::ObjectSubclassIsExt};

use serde::Deserialize;
use zvariant::{OwnedObjectPath, OwnedValue, Value};

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

impl UnitInfo {
    pub fn from_listed_unit(listed_unit: ListedLoadedUnit, level: UnitDBusLevel) -> Self {
        // let this_object: Self = glib::Object::new();
        let this_object: Self = glib::Object::builder()
            .property("primary", &listed_unit.primary_unit_name)
            .build();
        let imp = this_object.imp();
        imp.init_from_listed_unit(listed_unit, level);
        this_object
    }

    pub fn from_unit_file(unit_file: ListedUnitFile, level: UnitDBusLevel) -> Self {
        // let this_object: Self = glib::Object::new();
        let this_object: Self = glib::Object::builder()
            .property("primary", unit_file.unit_primary_name())
            .build();
        this_object.imp().init_from_unit_file(unit_file, level);
        this_object
    }

    pub fn update_from_loaded_unit(&self, listed_unit: ListedLoadedUnit) {
        self.imp().update_from_listed_unit(listed_unit);
    }

    pub fn update_from_unit_info(&self, update: UpdatedUnitInfo) {
        self.imp().update_from_unit_info(self, update);
    }

    pub fn update_from_unit_file(&self, unit_file: ListedUnitFile) {
        self.imp().update_from_unit_file(unit_file);
    }

    pub fn debug(&self) -> String {
        format!("{:#?}", *self.imp())
    }

    pub fn need_to_be_completed(&self) -> bool {
        self.imp().need_to_be_completed()
    }

    pub fn fill_property_values(&self, property_value_list: Vec<UnitPropertySetter>) {
        for setter in property_value_list {
            match setter {
                UnitPropertySetter::FileState(unit_file_status) => {
                    self.set_enable_status(unit_file_status)
                }
                UnitPropertySetter::Description(description) => self.set_description(description),
                UnitPropertySetter::ActiveState(active_state) => {
                    self.set_active_state(active_state)
                }
                UnitPropertySetter::LoadState(load_state) => self.set_load_state(load_state),
                UnitPropertySetter::FragmentPath(_) => todo!(),
                UnitPropertySetter::UnitFilePreset(preset) => self.set_preset(preset),
                UnitPropertySetter::SubState(substate) => self.set_sub_state(substate),
                UnitPropertySetter::Custom(quark, owned_value) => {
                    println!("cust {:?}", owned_value);
                    self.insert_unit_property_value(quark, owned_value)
                }
            }
        }
    }

    fn insert_unit_property_value(&self, quark: Quark, value: OwnedValue) {
        //let value_ref = &value as &Value;
        match &value as &Value {
            Value::Bool(b) => unsafe { self.set_qdata(quark, *b) },
            Value::U8(i) => unsafe { self.set_qdata(quark, *i) },
            Value::I16(i) => unsafe { self.set_qdata(quark, *i) },
            Value::U16(i) => unsafe { self.set_qdata(quark, *i) },
            Value::I32(i) => unsafe { self.set_qdata(quark, *i) },
            Value::U32(i) => unsafe { self.set_qdata(quark, *i) },
            Value::I64(i) => unsafe { self.set_qdata(quark, *i) },
            Value::U64(i) => unsafe { self.set_qdata(quark, *i) },
            Value::F64(i) => unsafe { self.set_qdata(quark, *i) },
            Value::Str(s) => {
                if s.is_empty() {
                    unsafe { self.steal_qdata::<String>(quark) };
                } else {
                    unsafe { self.set_qdata(quark, s.to_string()) };
                }
            }
            Value::Signature(s) => unsafe { self.set_qdata(quark, s.to_string()) },
            Value::ObjectPath(op) => unsafe { self.set_qdata(quark, op.to_string()) },
            Value::Value(val) => unsafe { self.set_qdata(quark, val.to_string()) },
            Value::Array(array) => {
                if array.is_empty() {
                    unsafe { self.steal_qdata::<String>(quark) };
                } else {
                    let mut d_str = String::from("");

                    let mut it = array.iter().peekable();
                    while let Some(mi) = it.next() {
                        if let Some(str_value) = convert_to_string(mi) {
                            d_str.push_str(&str_value);
                        }
                        if it.peek().is_some() {
                            d_str.push_str(", ");
                        }
                    }

                    unsafe { self.set_qdata(quark, d_str) };
                }
            }
            Value::Dict(d) => {
                let mut it = d.iter().peekable();
                if it.peek().is_none() {
                    unsafe { self.steal_qdata::<String>(quark) };
                } else {
                    let mut d_str = String::from("{ ");

                    for (mik, miv) in it {
                        if let Some(k) = convert_to_string(mik) {
                            d_str.push_str(&k);
                        }
                        d_str.push_str(" : ");

                        if let Some(v) = convert_to_string(miv) {
                            d_str.push_str(&v);
                        }
                    }
                    d_str.push_str(" }");

                    unsafe { self.set_qdata(quark, d_str) };
                }
            }
            Value::Structure(stc) => {
                let mut it = stc.fields().iter().peekable();

                if it.peek().is_none() {
                    unsafe { self.steal_qdata::<String>(quark) };
                } else {
                    let v: Vec<String> = it
                        .filter_map(|v| convert_to_string(v))
                        .filter(|s| !s.is_empty())
                        .collect();
                    let d_str = v.join(", ");

                    unsafe { self.set_qdata(quark, d_str) };
                }
            }
            Value::Fd(fd) => unsafe { self.set_qdata(quark, fd.to_string()) },
            //Value::Maybe(maybe) => (maybe.to_string(), false),
        }
    }
}

mod imp {
    use std::{
        cell::{Cell, OnceCell, RefCell},
        str::FromStr,
    };

    use base::enums::UnitDBusLevel;
    use glib::{
        self,
        object::ObjectExt,
        subclass::{object::*, types::ObjectSubclass},
    };

    use crate::{
        UpdatedUnitInfo,
        data::ListedLoadedUnit,
        enums::{ActiveState, LoadState, Preset, UnitFileStatus, UnitType},
        sysdbus::ListedUnitFile,
    };

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitInfo)]
    pub struct UnitInfoImpl {
        #[property(get, construct_only, set = Self::set_primary)]
        pub(super) primary: OnceCell<String>,

        #[property(get = Self::get_display_name, type = String)]
        display_name: OnceCell<u32>,

        #[property(get, default)]
        unit_type: Cell<UnitType>,

        #[property(get, set)]
        pub(super) description: RefCell<Option<String>>,

        #[property(get, set, default)]
        pub(super) load_state: Cell<LoadState>,

        #[property(get, set, builder(ActiveState::Unknown))]
        pub(super) active_state: Cell<ActiveState>,

        #[property(get, set)]
        pub(super) sub_state: RefCell<String>,

        #[property(get)]
        pub(super) followed_unit: RefCell<String>,

        //#[property(get = Self::has_object_path, name = "pathexists", type = bool)]
        #[property(get=Self::get_unit_path, type = String)]
        pub(super) object_path: OnceCell<String>,
        #[property(get, set, nullable, default = None)]
        pub(super) file_path: RefCell<Option<String>>,
        #[property(get, set, default)]
        pub(super) enable_status: Cell<UnitFileStatus>,

        #[property(get, set, default)]
        pub(super) dbus_level: Cell<UnitDBusLevel>,

        #[property(get, set, default)]
        pub(super) preset: Cell<Preset>,
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
            listed_unit: super::ListedLoadedUnit,
            dbus_level: UnitDBusLevel,
        ) {
            self.dbus_level.replace(dbus_level);
            self.update_from_listed_unit(listed_unit);
        }

        pub(super) fn update_from_listed_unit(&self, listed_unit: ListedLoadedUnit) {
            let active_state: ActiveState = listed_unit.active_state.as_str().into();

            //self.set_primary(listed_unit.primary_unit_name);
            self.active_state.replace(active_state);

            let description = if listed_unit.description.is_empty() {
                None
            } else {
                Some(listed_unit.description)
            };

            self.description.replace(description);
            let load_state: LoadState = listed_unit.load_state.as_str().into();
            self.load_state.replace(load_state);
            self.sub_state.replace(listed_unit.sub_state);
            self.followed_unit.replace(listed_unit.followed_unit);
        }

        pub(super) fn init_from_unit_file(&self, unit_file: ListedUnitFile, level: UnitDBusLevel) {
            self.dbus_level.replace(level);
            self.update_from_unit_file(unit_file)
        }

        pub(super) fn update_from_unit_file(&self, unit_file: ListedUnitFile) {
            self.file_path.replace(Some(unit_file.unit_file_path));
            let status = UnitFileStatus::from_str(&unit_file.enablement_status)
                .unwrap_or(UnitFileStatus::default());
            self.enable_status.replace(status);
        }

        fn set_primary(&self, primary: String) {
            let mut split_char_index = primary.len();
            for (i, c) in primary.chars().rev().enumerate() {
                if c == '.' {
                    split_char_index -= i;
                    break;
                }
            }

            // let display_name = primary[..split_char_index - 1].to_owned();
            self.display_name.set((split_char_index - 1) as u32);

            let unit_type = UnitType::new(&primary[(split_char_index)..]);
            self.unit_type.set(unit_type);

            self.primary.set(primary);
        }

        pub fn get_display_name(&self) -> String {
            let index = *self.display_name.get_or_init(|| unreachable!()) as usize;
            let s = &self.primary.get().expect("Being set")[..index];
            s.to_owned()
        }

        pub fn update_from_unit_info(&self, unit: &super::UnitInfo, update: UpdatedUnitInfo) {
            // self.object_path.replace(Some(update.object_path));

            self.description.replace(update.description);

            if let Some(sub_state) = update.sub_state {
                self.sub_state.replace(sub_state);
            }

            if let Some(active_state) = update.active_state {
                self.active_state.replace(active_state);
            }

            if let Some(unit_file_preset) = update.unit_file_preset {
                let preset: Preset = unit_file_preset.into();
                unit.set_preset(preset);
            }

            if let Some(load_state) = update.load_state {
                unit.set_load_state(load_state);
            }

            if let Some(fragment_path) = update.fragment_path {
                self.file_path.replace(Some(fragment_path));
            }

            if let Some(enablement_status) = update.enablement_status {
                self.enable_status.replace(enablement_status);
            }
        }

        fn get_unit_path(&self) -> String {
            let object_path = self.object_path.get_or_init(|| {
                let primary = self.primary.get_or_init(|| unreachable!());
                crate::sysdbus::unit_dbus_path_from_name(primary)
            });
            object_path.clone()
        }

        pub fn need_to_be_completed(&self) -> bool {
            self.description.borrow().is_none() || self.preset.get() == Preset::UnSet
            // || self.load_state.get() == LoadState::Unknown
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

#[derive(Deserialize, zvariant::Type, PartialEq, Debug)]
pub struct ListedLoadedUnit {
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

pub fn convert_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Bool(b) => Some(b.to_string()),
        Value::U8(i) => Some(i.to_string()),
        Value::I16(i) => Some(i.to_string()),
        Value::U16(i) => Some(i.to_string()),
        Value::I32(i) => Some(i.to_string()),
        Value::U32(i) => Some(i.to_string()),
        Value::I64(i) => Some(i.to_string()),
        Value::U64(i) => Some(i.to_string()),
        Value::F64(i) => Some(i.to_string()),
        Value::Str(s) => Some(s.to_string()),
        Value::Signature(s) => Some(s.to_string()),
        Value::ObjectPath(op) => Some(op.to_string()),
        Value::Value(v) => Some(v.to_string()),
        Value::Array(a) => {
            if a.is_empty() {
                None
            } else {
                let mut d_str = String::from("");

                let mut it = a.iter().peekable();
                while let Some(mi) = it.next() {
                    if let Some(v) = convert_to_string(mi) {
                        d_str.push_str(&v);
                    }
                    if it.peek().is_some() {
                        d_str.push_str(", ");
                    }
                }

                Some(d_str)
            }
        }
        Value::Dict(d) => {
            let mut it = d.iter().peekable();
            if it.peek().is_none() {
                None
            } else {
                let mut d_str = String::from("{ ");

                for (mik, miv) in it {
                    if let Some(k) = convert_to_string(mik) {
                        d_str.push_str(&k);
                    }
                    d_str.push_str(" : ");

                    if let Some(v) = convert_to_string(miv) {
                        d_str.push_str(&v);
                    }
                }
                d_str.push_str(" }");
                Some(d_str)
            }
        }
        Value::Structure(stc) => {
            let mut it = stc.fields().iter().peekable();

            if it.peek().is_none() {
                None
            } else {
                let mut d_str = String::from("");

                while let Some(mi) = it.next() {
                    if let Some(v) = convert_to_string(mi) {
                        d_str.push_str(&v);
                    }

                    if it.peek().is_some() {
                        d_str.push_str(", ");
                    }
                }

                Some(d_str)
            }
        }
        Value::Fd(fd) => Some(fd.to_string()),
        //Value::Maybe(maybe) => (maybe.to_string(), false),
    }
}

pub enum UnitPropertyGetter<'a> {
    Managed(),
    Custom(UnitType, &'a str),
}

pub enum UnitPropertySetter {
    FileState(UnitFileStatus),
    Description(String),
    ActiveState(ActiveState),
    LoadState(LoadState),
    FragmentPath(String),
    UnitFilePreset(Preset),
    SubState(String),
    Custom(Quark, OwnedValue),
}
