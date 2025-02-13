use std::cmp::Ordering;

use super::enums::ActiveState;
use crate::widget::preferences::data::DbusLevel;
use gtk::{
    glib::{self},
    subclass::prelude::*,
};

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

impl UnitInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        primary: &str,
        description: &str,
        load_state: &str,
        active_state: ActiveState,
        sub_state: &str,
        followed_unit: &str,
        object_path: Option<&str>,
        dbus_level: DbusLevel,
    ) -> Self {
        let this_object: Self = glib::Object::new();
        let imp: &imp::UnitInfoImpl = this_object.imp();
        imp.assign_new(
            primary,
            description,
            load_state,
            active_state,
            sub_state,
            followed_unit,
            object_path,
            dbus_level,
        );

        this_object
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

    use crate::{systemd::enums::ActiveState, widget::preferences::data::DbusLevel};

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
        pub(super) enable_status: RwLock<u32>,

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
        #[allow(clippy::too_many_arguments)]
        pub fn assign_new(
            &self,
            primary: &str,
            description: &str,
            load_state: &str,
            active_state: ActiveState,
            sub_state: &str,
            followed_unit: &str,
            object_path: Option<&str>,
            level: DbusLevel,
        ) {
            self.set_primary(primary.to_owned());
            self.set_active_state(active_state);

            *self.description.write().unwrap() = description.to_owned();
            *self.load_state.write().unwrap() = load_state.to_owned();
            *self.sub_state.write().unwrap() = sub_state.to_owned();
            *self.followed_unit.write().unwrap() = followed_unit.to_owned();
            *self.object_path.write().unwrap() = object_path.map(str::to_owned);
            *self.level.write().unwrap() = level;
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
