use crate::data::{ListedLoadedUnit, PRIMARY, UnitInfo};

use base::enums::UnitDBusLevel;
use glib::{self, object::ObjectExt, subclass::types::ObjectSubclassIsExt};

glib::wrapper! {
    pub struct SocketUnitInfo(ObjectSubclass<imp::SocketUnitInfoImpl>)
    @extends UnitInfo;
}

impl SocketUnitInfo {
    pub fn from_unit_socket(unit: &UnitInfo, socket_listen_idx: usize) -> Self {
        // let this_object: Self = glib::Object::new();

        let this_object: Self = glib::Object::builder()
            .property(PRIMARY, unit.primary())
            .build();
        this_object.imp().init(unit, socket_listen_idx);
        this_object
    }

    pub fn get_custom_property<T: 'static>(&self, key: glib::Quark) -> Option<&T> {
        self.imp()
            .base_unit
            .borrow()
            .clone()
            .map(|a| unsafe { a.qdata::<T>(key) }.map(|value_ptr| unsafe { value_ptr.as_ref() }))
            .flatten()
    }
}

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use base::enums::UnitDBusLevel;
    use glib::{
        self,
        object::{IsA, ObjectExt},
        subclass::{
            object::*,
            types::{IsSubclassable, ObjectSubclass},
        },
    };

    use crate::{
        data::UnitInfo,
        enums::{ActiveState, UnitType},
    };

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::SocketUnitInfo)]
    pub struct SocketUnitInfoImpl {
        #[property(get, construct_only, set = Self::set_primary)]
        pub(super) primary: OnceCell<String>,

        #[property(get = Self::get_display_name, type = String)]
        display_name: OnceCell<u32>,
        #[property(get, builder(UnitType::Socket))]
        unit_type: Cell<UnitType>,

        #[property(get=Self::get_active_state, set,  default)]
        pub active_state: Cell<ActiveState>,

        #[property(get, set, default)]
        dbus_level: Cell<UnitDBusLevel>,

        socket_listen_idx: Cell<usize>,

        #[property(get=Self::get_unit_path, type = String)]
        pub(super) object_path: OnceCell<String>,

        pub(super) base_unit: RefCell<Option<UnitInfo>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SocketUnitInfoImpl {
        const NAME: &'static str = "SocketUnitInfo";
        type Type = super::SocketUnitInfo;
        type ParentType = UnitInfo;

        fn new() -> Self {
            Default::default()
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SocketUnitInfoImpl {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl SocketUnitInfoImpl {
        pub(super) fn init(&self, unit: &UnitInfo, socket_listen_idx: usize) {
            self.dbus_level.replace(unit.dbus_level());
            self.socket_listen_idx.replace(socket_listen_idx);
            self.base_unit.replace(Some(unit.clone()));
        }

        pub fn get_display_name(&self) -> String {
            let index = *self.display_name.get_or_init(|| unreachable!()) as usize;
            let s = &self.primary.get().expect("Being set")[..index];
            s.to_owned()
        }

        pub fn get_active_state(&self) -> ActiveState {
            let u = self.base_unit.borrow();
            let u = u.as_ref().unwrap();
            u.active_state()
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

        fn get_unit_path(&self) -> String {
            let u = self.base_unit.borrow();
            let u = u.as_ref().unwrap();
            u.object_path()
        }
    }

    pub trait UnitInfoImplTr: ObjectImpl + ObjectSubclass<Type: IsA<UnitInfo>> {}

    unsafe impl<T: UnitInfoImplTr> IsSubclassable<T> for UnitInfo {}

    impl UnitInfoImplTr for SocketUnitInfoImpl {}
}
