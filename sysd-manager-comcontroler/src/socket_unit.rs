use crate::data::{ListedLoadedUnit, UnitInfo};

use base::enums::UnitDBusLevel;
use glib::{self};

glib::wrapper! {
    pub struct SocketUnitInfo(ObjectSubclass<imp::SocketUnitInfoImpl>)
    @extends UnitInfo;
}

impl SocketUnitInfo {
    pub fn from_listed_unit(listed_unit: ListedLoadedUnit, level: UnitDBusLevel) -> Self {
        // let this_object: Self = glib::Object::new();
        let this_object: Self = glib::Object::builder()
            .property("primary", &listed_unit.primary_unit_name)
            .build();
        let sub: UnitInfo = this_object.clone().into();
        sub.init_from_listed_unit(listed_unit, level);
        this_object
    }
}

mod imp {
    use std::cell::Cell;

    use glib::{
        self,
        object::{IsA, ObjectExt},
        subclass::{
            object::*,
            types::{IsSubclassable, ObjectSubclass},
        },
    };

    use crate::data::UnitInfo;

    #[derive(Debug, glib::Properties, Default)]
    #[properties(wrapper_type = super::SocketUnitInfo)]
    pub struct SocketUnitInfoImpl {
        #[property(get, set)]
        pub(super) preset: Cell<u8>,
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

    impl ObjectImpl for SocketUnitInfoImpl {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl SocketUnitInfoImpl {}

    pub trait UnitInfoImplTr: ObjectImpl + ObjectSubclass<Type: IsA<UnitInfo>> {}

    unsafe impl<T: UnitInfoImplTr> IsSubclassable<T> for UnitInfo {}

    impl UnitInfoImplTr for SocketUnitInfoImpl {}
}
