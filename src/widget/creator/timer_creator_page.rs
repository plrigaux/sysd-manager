use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

use crate::widget::creator::UnitCreatorWindow;

glib::wrapper! {

    pub struct TimerCreatorPage(ObjectSubclass<imp::TimerCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl TimerCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>) -> Self {
        let obj: TimerCreatorPage = glib::Object::new();
        let _ = obj.imp().window.set(window);
        obj.imp().update_from_unit_info();
        obj
    }
}

mod imp {

    use super::TimerCreatorPage;
    use crate::{
        upgrade, upgrade_opt,
        widget::creator::{UnitCreateType, UnitCreatorWindow},
    };
    use adw::{prelude::ComboRowExt, subclass::prelude::*};
    use glib::WeakRef;
    use gtk::{
        glib::{self},
        prelude::ObjectExt,
    };
    use std::cell::{Cell, OnceCell};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(resource = "/io/github/plrigaux/sysd-manager/timer_creator_page.ui")]
    #[properties(wrapper_type = super::TimerCreatorPage)]
    pub struct TimerCreatorPageImp {
        #[property(get, set, default)]
        creation_type: Cell<UnitCreateType>,

        #[template_child]
        trigger_unit: TemplateChild<adw::ComboRow>,

        pub(super) window: OnceCell<WeakRef<UnitCreatorWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimerCreatorPageImp {
        const NAME: &'static str = "TimerCreatorPage";
        type Type = TimerCreatorPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.bind_template();
            //klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TimerCreatorPageImp {
        fn constructed(&self) {
            self.parent_constructed();

            self.trigger_unit.connect_selected_item_notify(|a| {
                println!("Conn idc {}", a.selected());
            });
        }
    }

    impl TimerCreatorPageImp {
        pub(super) fn update_from_unit_info(&self) {
            let window = upgrade_opt!(self.window.get());

            let set = window.imp().get_trigger_units();

            let mut vec = set
                .iter()
                .filter(|s| !s.ends_with(".timer"))
                .map(|s| s.as_ref())
                .collect::<Vec<_>>();
            vec.push(""); //for unselect
            vec.sort();

            let model = gtk::StringList::new(&vec);
            // self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
            self.trigger_unit.set_model(Some(&model));
            self.trigger_unit.set_selected(gtk::INVALID_LIST_POSITION);
            println!("sel {}", self.trigger_unit.selected());
        }
    }

    impl WidgetImpl for TimerCreatorPageImp {}

    impl NavigationPageImpl for TimerCreatorPageImp {}
}
