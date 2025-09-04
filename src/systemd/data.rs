use std::{cmp::Ordering, fmt::Debug};

use super::{
    SystemdUnitFile, UpdatedUnitInfo,
    enums::{ActiveState, EnablementStatus, LoadState, Preset, UnitDBusLevel},
    sysdbus::LUnit,
};

use gtk::{
    glib::{self},
    subclass::prelude::*,
};
use serde::Deserialize;
use zvariant::Type;

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

    pub fn from_listed_unit(listed_unit: &LUnit, level: UnitDBusLevel) -> Self {
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
        self.set_object_path(update.object_path);

        if let Some(description) = update.description {
            self.set_description(description);
        }

        if let Some(sub_state) = update.sub_state {
            self.set_sub_state(sub_state);
        }

        if let Some(active_state) = update.active_state {
            self.set_active_state(active_state);
        }

        if let Some(unit_file_preset) = update.unit_file_preset {
            self.set_preset(&unit_file_preset);
        }

        if let Some(load_state) = update.load_state {
            self.set_load_state(&load_state);
        }

        if let Some(fragment_path) = update.fragment_path {
            self.set_file_path(Some(fragment_path));
        }

        if let Some(enablement_status) = update.enablement_status {
            let enablement_status: u8 = enablement_status.into();
            self.set_enable_status(enablement_status);
        }
    }

    pub fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
        self.imp().update_from_unit_file(unit_file);
    }

    pub fn active_state(&self) -> ActiveState {
        self.imp().active_state()
    }

    pub fn set_active_state(&self, state: ActiveState) {
        self.imp().set_active_state(state)
    }

    pub fn preset(&self) -> Preset {
        self.imp().preset()
    }

    pub fn preset_str(&self) -> &'static str {
        self.imp().preset().as_str()
    }

    pub fn set_preset(&self, preset: &str) {
        self.imp().set_preset(preset)
    }

    pub fn dbus_level(&self) -> UnitDBusLevel {
        *self.imp().level.read().unwrap()
    }

    pub fn dbus_level_str(&self) -> &'static str {
        self.dbus_level().as_str()
    }

    pub fn enable_status_enum(&self) -> EnablementStatus {
        self.enable_status().into()
    }

    pub fn enable_status_str(&self) -> &'static str {
        self.enable_status_enum().as_str()
    }

    pub fn load_state(&self) -> LoadState {
        self.imp().load_state()
    }
    pub fn load_state_str(&self) -> &'static str {
        self.load_state().as_str()
    }

    pub fn set_load_state(&self, value: &str) {
        self.imp().set_load_state(value);
    }

    pub fn debug(&self) -> String {
        format!("{:#?}", *self.imp())
    }
}

mod imp {
    use std::sync::RwLock;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    use crate::systemd::{
        SystemdUnitFile,
        enums::{ActiveState, LoadState, Preset, UnitDBusLevel},
        sysdbus::LUnit,
    };

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitInfo)]
    pub struct UnitInfoImpl {
        #[property(get, set = Self::set_primary )]
        pub(super) primary: RwLock<String>,
        #[property(get)]
        display_name: RwLock<String>,
        #[property(get)]
        unit_type: RwLock<String>,
        #[property(get, set)]
        pub(super) description: RwLock<String>,

        #[property(get=Self::load_state_num, name="load-state-num", type = u8)]
        pub(super) load_state: RwLock<LoadState>,

        #[property(get, set=Self::set_active_state_num)]
        pub(super) active_state_num: RwLock<u8>,
        pub(super) active_state: RwLock<ActiveState>,

        #[property(get, set)]
        pub(super) sub_state: RwLock<String>,
        #[property(get)]
        pub(super) followed_unit: RwLock<String>,

        //#[property(get = Self::has_object_path, name = "pathexists", type = bool)]
        #[property(get, set)]
        pub(super) object_path: RwLock<Option<String>>,
        #[property(get, set, nullable, default = None)]
        pub(super) file_path: RwLock<Option<String>>,
        #[property(get, set, default = 0)]
        pub(super) enable_status: RwLock<u8>,

        pub(super) level: RwLock<UnitDBusLevel>,

        #[property(get=Self::preset_num, name="preset-num", type = u8)]
        pub(super) preset: RwLock<Preset>,
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
        pub(super) fn init_from_listed_unit(&self, listed_unit: &LUnit, dbus_level: UnitDBusLevel) {
            let active_state: ActiveState = listed_unit.active_state.into();

            self.set_primary(listed_unit.primary_unit_name.to_owned());
            self.set_active_state(active_state);

            *self.description.write().unwrap() = listed_unit.description.to_owned();
            self.set_load_state(listed_unit.load_state);
            *self.sub_state.write().unwrap() = listed_unit.sub_state.to_owned();
            *self.followed_unit.write().unwrap() = listed_unit.followed_unit.to_owned();
            *self.object_path.write().unwrap() = Some(listed_unit.unit_object_path.to_string());
            *self.level.write().unwrap() = dbus_level;
        }

        pub(super) fn init_from_unit_file(&self, unit_file: SystemdUnitFile) {
            self.set_primary(unit_file.full_name);
            self.set_active_state(ActiveState::Unknown);
            *self.level.write().unwrap() = unit_file.level;
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code as u8;
        }

        pub(super) fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code as u8
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

            let unit_type = primary[(split_char_index)..].to_owned();
            *self.unit_type.write().expect("set_primary unit_type") = unit_type;

            *self.primary.write().expect("set_primary primary") = primary;
        }

        pub fn set_active_state_num(&self, state: u8) {
            *self
                .active_state_num
                .write()
                .expect("set_active_state active_state") = state;
        }

        pub fn set_active_state(&self, state: ActiveState) {
            *self
                .active_state
                .write()
                .expect("set_active_state active_state") = state;

            //call this way to make binding works
            self.obj().set_active_state_num(state as u8)
        }

        pub fn active_state(&self) -> ActiveState {
            *self.active_state.read().expect("get active_state")
        }

        pub fn preset(&self) -> Preset {
            *self.preset.read().expect("get preset")
        }

        pub fn set_preset(&self, preset: &str) {
            let preset = preset.into();

            *self.preset.write().expect("set_preset preset") = preset;
        }

        pub fn preset_num(&self) -> u8 {
            self.preset().discriminant()
        }

        pub fn load_state_num(&self) -> u8 {
            self.load_state().discriminant()
        }

        pub fn load_state(&self) -> LoadState {
            *self.load_state.read().expect("get load_state")
        }

        pub fn set_load_state(&self, value: &str) {
            *self.load_state.write().expect("set_load_state") = value.into();
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
