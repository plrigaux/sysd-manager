use super::enums::ActiveState;
use crate::gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfoImpl>);
}

impl UnitInfo {
    pub fn new(
        primary: &str,
        description: &str,
        load_state: &str,
        active_state: ActiveState,
        sub_state: &str,
        followed_unit: &str,
        object_path: &str,
    ) -> Self {
        let this_object: Self = glib::Object::new();
        let imp: &imp::UnitInfoImpl = this_object.imp();

        imp.set_primary(primary.to_owned());
        *imp.description.write().unwrap() = description.to_owned();
        *imp.load_state.write().unwrap() = load_state.to_owned();
        *imp.active_state.write().unwrap() = active_state as u32;
        let icon_name = active_state.icon_name().map(|s| s.to_string());
        *imp.active_state_icon.write().unwrap() = icon_name;
        *imp.sub_state.write().unwrap() = sub_state.to_owned();
        *imp.followed_unit.write().unwrap() = followed_unit.to_owned();
        *imp.object_path.write().unwrap() = object_path.to_owned();

        this_object
    }
}

pub mod imp {
    use std::sync::RwLock;

    use gtk::{glib, prelude::*, subclass::prelude::*};

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
        #[property(get, set)]
        pub(super) active_state: RwLock<u32>,
        #[property(get, set, nullable)]
        pub(super) active_state_icon: RwLock<Option<String>>,
        #[property(get)]
        pub(super) sub_state: RwLock<String>,
        #[property(get)]
        pub(super) followed_unit: RwLock<String>,

        #[property(get = Self::has_object_path, name = "pathexist", type = bool)]
        #[property(get, set)]
        pub(super) object_path: RwLock<String>,
        #[property(get, set, nullable, default = None)]
        pub(super) file_path: RwLock<Option<String>>,
        #[property(get, set, default = 0)]
        pub(super) enable_status: RwLock<u32>,
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
        pub fn set_primary(&self, primary: String) {
            let mut split_char_index = primary.len();
            for (i, c) in primary.chars().enumerate() {
                if c == '.' {
                    split_char_index = i;
                }
            }

            let display_name = (&primary[..split_char_index]).to_owned();
            *self.display_name.write().unwrap() = display_name;

            let unit_type = (&primary[(split_char_index + 1)..]).to_owned();
            *self.unit_type.write().unwrap() = unit_type;

            *self.primary.write().unwrap() = primary;
        }

        pub fn has_object_path(&self) -> bool {
            let res = self.object_path.read();

            match res {
                Ok(a) => {
                    let empty_object_path = (*a).is_empty();
                    !empty_object_path
                }
                Err(_) => false,
            }
            //.unwrap().is_empty();
        }
    }
}
