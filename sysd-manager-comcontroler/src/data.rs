use std::{cmp::Ordering, fmt::Debug};

use super::{SystemdUnitFile, UpdatedUnitInfo};

use base::enums::UnitDBusLevel;
use glib::{self, subclass::types::ObjectSubclassIsExt};

use serde::Deserialize;
use zvariant::{OwnedObjectPath, Value};

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

// impl Default for UnitInfo {
//     fn default() -> Self {
//         UnitInfo::new()
//     }
// }

impl UnitInfo {
    // fn new() -> Self {
    //     let this_object: Self = glib::Object::new();
    //     this_object
    // }

    pub fn from_listed_unit(listed_unit: LUnit, level: UnitDBusLevel) -> Self {
        // let this_object: Self = glib::Object::new();
        let this_object: Self = glib::Object::builder()
            .property("primary", &listed_unit.primary_unit_name)
            .build();
        let imp = this_object.imp();
        imp.init_from_listed_unit(listed_unit, level);
        this_object
    }

    pub fn from_unit_file(unit_file: SystemdUnitFile) -> Self {
        // let this_object: Self = glib::Object::new();
        let this_object: Self = glib::Object::builder()
            .property("primary", &unit_file.full_name)
            .build();
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

    pub fn need_to_be_completed(&self) -> bool {
        self.imp().need_to_be_completed()
    }
}

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use base::enums::UnitDBusLevel;
    use glib::{
        self,
        object::ObjectExt,
        subclass::{object::*, types::ObjectSubclass},
    };

    use crate::{
        SystemdUnitFile, UpdatedUnitInfo,
        enums::{ActiveState, EnablementStatus, LoadState, Preset, UnitType},
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
        pub(super) enable_status: Cell<EnablementStatus>,

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
            listed_unit: super::LUnit,
            dbus_level: UnitDBusLevel,
        ) {
            let active_state: ActiveState = listed_unit.active_state.as_str().into();

            //self.set_primary(listed_unit.primary_unit_name);
            self.active_state.replace(active_state);

            let description = if !listed_unit.description.is_empty() {
                Some(listed_unit.description)
            } else {
                None
            };

            self.description.replace(description);
            let load_state: LoadState = listed_unit.load_state.as_str().into();
            self.load_state.replace(load_state);
            self.sub_state.replace(listed_unit.sub_state);
            self.followed_unit.replace(listed_unit.followed_unit);
            // let unit_object_path = Some(listed_unit.unit_object_path.to_string());
            // self.object_path.replace(unit_object_path);
            self.dbus_level.replace(dbus_level);
        }

        pub(super) fn init_from_unit_file(&self, unit_file: SystemdUnitFile) {
            // self.set_primary(unit_file.full_name);
            //self.set_active_state(ActiveState::Unknown);
            self.dbus_level.replace(unit_file.level);
            self.file_path.replace(Some(unit_file.file_path));
            self.enable_status.replace(unit_file.status_code);
        }

        pub(super) fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
            self.file_path.replace(Some(unit_file.file_path));
            self.enable_status.replace(unit_file.status_code);
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

        pub fn update_from_unit_info(&self, update: UpdatedUnitInfo) {
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
