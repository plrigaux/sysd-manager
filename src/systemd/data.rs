use super::ActiveState;
use crate::gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;

glib::wrapper! {
    pub struct UnitInfo(ObjectSubclass<imp::UnitInfo>);
}

impl UnitInfo {
    pub fn new(
        primary: &String,
        description: &String,
        load_state: &String,
        active_state: ActiveState,
        sub_state: &String,
        followed_unit: &String,
        object_path: String,
    ) -> Self {
        let this: Self = glib::Object::new();
        // this.
        let imp: &imp::UnitInfo = this.imp();

        imp.primary.replace(primary.to_owned());
        imp.description.replace(description.to_owned());
        imp.load_state.replace(load_state.to_owned());
        imp.active_state.replace(active_state as u32);
        imp.sub_state.replace(sub_state.to_owned());
        imp.followed_unit.replace(followed_unit.to_owned());
        imp.object_path.replace(object_path.to_owned());

        this
    }
}

pub mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::{glib, prelude::*, subclass::prelude::*};

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::UnitInfo)]
    pub struct UnitInfo {
        #[property(get, set = Self::set_primary )]
        pub(super) primary: RefCell<String>,
        #[property(get)]
        display_name: RefCell<String>,
        #[property(get)]
        unit_type: RefCell<String>,
        #[property(get)]
        pub(super) description: RefCell<String>,
        #[property(get)]
        pub(super) load_state: RefCell<String>,
        #[property(get, set)]
        pub(super) active_state: Cell<u32>,
        #[property(get)]
        pub(super) sub_state: RefCell<String>,
        #[property(get)]
        pub(super) followed_unit: RefCell<String>,
        #[property(get)]
        pub(super) object_path: RefCell<String>,
        #[property(get, set)]
        pub(super) file_path: RefCell<Option<String>>,
        #[property(get, set)]
        pub(super) enable_status: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnitInfo {
        const NAME: &'static str = "UnitInfo";
        type Type = super::UnitInfo;

        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for UnitInfo {}

    impl UnitInfo {
        pub fn set_primary(&self, primary: String) {
            let mut split_char_index = primary.len();
            for (i, c) in primary.chars().enumerate() {
                if c == '.' {
                    split_char_index = i;
                }
            }

            // self.separator.replace(split_char_index);
            self.primary.replace(primary.clone());
            self.display_name
                .replace((&primary[..split_char_index]).to_owned());
            self.unit_type
                .replace((&primary[(split_char_index + 1)..]).to_owned());
        }
    }
}

/*
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Unit {
    primary: String,
    description: String,
    load_state: String,
    active_state: ActiveState,
    sub_state: String,
    followed_unit: String,
    object_path: String,
    file_path: Option<String>,
    enable_status: Option<String>,
    separator: usize, /*     job_id: u32,
                      job_type: String,
                      job_object_path: String, */
}

/* const STATUS_ENABLED: &str = "enabled";
const STATUS_DISABLED: &str = "disabled"; */

impl Unit {
    pub fn new(
        primary: &String,
        description: &String,
        load_state: &String,
        active_state: ActiveState,
        sub_state: &String,
        followed_unit: &String,
        object_path: String,
    ) -> Self {
        let mut split_char_index = primary.len();
        for (i, c) in primary.chars().enumerate() {
            if c == '.' {
                split_char_index = i;
            }
        }

        Self {
            primary: primary.clone(),
            description: description.clone(),
            load_state: load_state.clone(),
            active_state: active_state,
            sub_state: sub_state.clone(),
            followed_unit: followed_unit.clone(),
            object_path: object_path.to_string(),
            enable_status: None,
            file_path: None,
            separator: split_char_index, /*                   job_id: job_id,
                                         job_type: job_type.clone(),
                                         job_object_path: job_object_path.to_string(), */
        }
    }
    pub fn primary(&self) -> &str {
        &self.primary
    }

    /*     pub fn is_enable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => STATUS_ENABLED == enable_status,
            None => false,
        }
    } */

    pub fn enable_status(&self) -> &str {
        match &self.enable_status {
            Some(enable_status) => &enable_status,
            None => "",
        }
    }

    pub fn display_name(&self) -> &str {
        &self.primary[..self.separator]
    }

    pub fn unit_type(&self) -> &str {
        &self.primary[(self.separator + 1)..]
    }

    pub fn active_state(&self) -> &str {
        &self.active_state.label()
    }

    pub fn set_active_state(&mut self, state : ActiveState)  {
        self.active_state = state;
    }

    pub fn active_state_icon(&self) -> &str {
        &self.active_state.icon_name()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn file_path(&self) -> Option<&String> {
        self.file_path.as_ref()
    }

    /*     fn is_enable_or_disable(&self) -> bool {
        match &self.enable_status {
            Some(enable_status) => {
                STATUS_ENABLED == enable_status || STATUS_DISABLED == enable_status
            }
            None => false,
        }
    } */
} */
