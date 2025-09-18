use std::{cmp::Ordering, fmt::Debug};

use super::{SystemdUnitFile, UpdatedUnitInfo, enums::UnitDBusLevel};

use gtk::{
    glib::{self},
    subclass::prelude::*,
};
use serde::Deserialize;
use zvariant::{OwnedObjectPath, OwnedValue, Type};

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

    pub fn set_property_values(&self, property_value_list: Vec<(String, OwnedValue)>) {
        self.imp().set_property_values(property_value_list);
    }
}

mod imp {
    use std::{cell::RefCell, collections::HashMap, sync::RwLock};

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
        pub(super) primary: RwLock<String>,
        #[property(get)]
        display_name: RwLock<String>,
        #[property(get, default)]
        unit_type: RwLock<UnitType>,
        #[property(get, set)]
        pub(super) description: RwLock<String>,

        #[property(get, set, default)]
        pub(super) load_state: RwLock<LoadState>,

        #[property(get, set, builder(ActiveState::Unknown))]
        pub(super) active_state: RwLock<ActiveState>,

        #[property(get, set)]
        pub(super) sub_state: RwLock<String>,
        #[property(get)]
        pub(super) followed_unit: RwLock<String>,

        //#[property(get = Self::has_object_path, name = "pathexists", type = bool)]
        #[property(get=Self::get_unit_path, type = String)]
        pub(super) object_path: RefCell<Option<String>>,
        #[property(get, set, nullable, default = None)]
        pub(super) file_path: RwLock<Option<String>>,
        #[property(get, set, default)]
        pub(super) enable_status: RwLock<EnablementStatus>,

        #[property(get, set, default)]
        pub(super) dbus_level: RwLock<UnitDBusLevel>,

        #[property(get, set, default)]
        pub(super) preset: RwLock<Preset>,

        custom_properties: RefCell<HashMap<String, OwnedValue>>,
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
            *self.active_state.write().unwrap() = active_state;

            *self.description.write().unwrap() = listed_unit.description;
            let load_state: LoadState = listed_unit.load_state.as_str().into();
            *self.load_state.write().unwrap() = load_state;
            *self.sub_state.write().unwrap() = listed_unit.sub_state;
            *self.followed_unit.write().unwrap() = listed_unit.followed_unit;
            let unit_object_path = Some(listed_unit.unit_object_path.to_string());
            self.object_path.replace(unit_object_path);
            *self.dbus_level.write().unwrap() = dbus_level;
        }

        pub(super) fn init_from_unit_file(&self, unit_file: SystemdUnitFile) {
            self.set_primary(unit_file.full_name);
            //self.set_active_state(ActiveState::Unknown);
            *self.dbus_level.write().unwrap() = unit_file.level;
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code;
        }

        pub(super) fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code;
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
            *self.display_name.write().expect("set_primary display_name") = display_name;

            let unit_type = UnitType::new(&primary[(split_char_index)..]);
            *self.unit_type.write().expect("set_primary unit_type") = unit_type;

            *self.primary.write().expect("set_primary primary") = primary;
        }

        pub fn update_from_unit_info(&self, update: UpdatedUnitInfo) {
            self.object_path.replace(Some(update.object_path));

            if let Some(description) = update.description {
                *self.description.write().unwrap() = description;
            }

            if let Some(sub_state) = update.sub_state {
                *self.sub_state.write().unwrap() = sub_state;
            }

            if let Some(active_state) = update.active_state {
                *self.active_state.write().unwrap() = active_state;
            }

            if let Some(unit_file_preset) = update.unit_file_preset {
                let preset: Preset = unit_file_preset.into();
                *self.preset.write().unwrap() = preset;
            }

            if let Some(load_state) = update.load_state {
                *self.load_state.write().unwrap() = load_state;
            }

            if let Some(fragment_path) = update.fragment_path {
                *self.file_path.write().unwrap() = Some(fragment_path);
            }

            if let Some(enablement_status) = update.enablement_status {
                *self.enable_status.write().unwrap() = enablement_status;
            }
        }

        fn get_unit_path(&self) -> String {
            if let Some(a) = &*self.object_path.borrow() {
                a.clone()
            } else {
                let primary = &*self.primary.read().unwrap();
                let object_path = sysdbus::unit_dbus_path_from_name(primary);
                self.object_path.replace(Some(object_path.clone()));
                object_path
            }
        }

        pub fn set_property_values(&self, _property_value_list: Vec<(String, OwnedValue)>) {
            //
            /*   match *asdf {
                           Some(a) => todo!(),
                           None => todo!(),
                       }
            */
            /* let custom_properties = match *self.custom_properties.get_mut().unwrap() {
                Some(custom_properties) => custom_properties,
                None => {
                    let custom_properties: HashMap<String, OwnedValue> =
                        HashMap::with_capacity(property_value_list.len());
                    *self.custom_properties.write().unwrap() = Some(custom_properties);

                    custom_properties
                }
            }; */
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
