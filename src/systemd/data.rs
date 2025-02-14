use std::cmp::Ordering;

use super::{enums::ActiveState, sysdbus::LUnit, SystemdUnitFile};
use crate::widget::preferences::data::DbusLevel;
use gtk::{
    glib::{self},
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

impl UnitInfo {
    pub fn from_listed_unit(listed_unit: &LUnit, level: DbusLevel) -> Self {
        let this_object: Self = glib::Object::new();
        let imp = this_object.imp();
        imp.init_from_listed_unit(listed_unit, level);
        this_object
    }

    pub fn from_unit_file(unit_file: SystemdUnitFile, level: DbusLevel) -> Self {
        let this_object: Self = glib::Object::new();
        let imp: &imp::UnitInfoImpl = this_object.imp();
        imp.init_from_unit_file(unit_file, level);
        this_object
    }

    pub fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
        let imp: &imp::UnitInfoImpl = self.imp();
        imp.update_from_unit_file(unit_file);
    }

    pub fn active_state(&self) -> ActiveState {
        self.imp().active_state()
    }

    pub fn set_active_state(&self, state: ActiveState) {
        self.imp().set_active_state(state)
    }

    pub fn dbus_level(&self) -> DbusLevel {
        *self.imp().level.read().unwrap()
    }
}

mod imp {
    use std::sync::RwLock;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    use crate::{
        systemd::{enums::ActiveState, sysdbus::LUnit, SystemdUnitFile},
        widget::preferences::data::DbusLevel,
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
        #[property(get)]
        pub(super) load_state: RwLock<String>,

        #[property(get, set=Self::set_active_state_num)]
        pub(super) active_state_num: RwLock<u8>,
        #[property(get)]
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

        pub(super) active_state: RwLock<ActiveState>,

        pub(super) level: RwLock<DbusLevel>,
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
        pub(super) fn init_from_listed_unit(&self, listed_unit: &LUnit, dbus_level: DbusLevel) {
            let active_state: ActiveState = listed_unit.active_state.into();

            self.set_primary(listed_unit.primary_unit_name.to_owned());
            self.set_active_state(active_state);

            *self.description.write().unwrap() = listed_unit.description.to_owned();
            *self.load_state.write().unwrap() = listed_unit.load_state.to_owned();
            *self.sub_state.write().unwrap() = listed_unit.sub_state.to_owned();
            *self.followed_unit.write().unwrap() = listed_unit.followed_unit.to_owned();
            *self.object_path.write().unwrap() = Some(listed_unit.unit_object_path.to_string());
            *self.level.write().unwrap() = dbus_level;
        }

        pub(super) fn init_from_unit_file(&self, unit_file: SystemdUnitFile, level: DbusLevel) {
            self.set_primary(unit_file.full_name);
            self.set_active_state(ActiveState::Unknown);
            *self.level.write().unwrap() = level;
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code as u8;
        }

        pub(super) fn update_from_unit_file(&self, unit_file: SystemdUnitFile) {
            *self.file_path.write().unwrap() = Some(unit_file.path);
            *self.enable_status.write().unwrap() = unit_file.status_code as u8
        }

        pub fn set_primary(&self, primary: String) {
            let mut split_char_index = primary.len();
            for (i, c) in primary.chars().enumerate() {
                if c == '.' {
                    split_char_index = i;
                }
            }

            let display_name = primary[..split_char_index].to_owned();
            *self.display_name.write().expect("set_primary display_name") = display_name;

            let unit_type = primary[(split_char_index + 1)..].to_owned();
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
