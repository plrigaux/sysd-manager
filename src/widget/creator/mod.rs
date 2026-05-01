mod imp;
mod service_creator_page;
mod timer_creator_page;
use crate::widget::app_window::AppWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib::{self};
use tracing::warn;

glib::wrapper! {

    pub struct UnitCreatorWindow(ObjectSubclass<imp::UnitCreatorWindowImp>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget,
    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UnitCreatorWindow {
    pub fn new(app_window: &AppWindow) -> Self {
        let obj: UnitCreatorWindow = glib::Object::new();
        let _ = obj.imp().app_window.set(app_window.clone());
        obj
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default, Hash)]
#[enum_type(name = "UnitCreateType")]
pub enum UnitCreateType {
    #[default]
    Service,
    Timer,
    TimerService,
    Unknown,
}

impl UnitCreateType {
    pub fn max_sufix_len(&self) -> usize {
        match self {
            UnitCreateType::Service => ".service".len(),
            UnitCreateType::Timer => ".timer".len(),
            UnitCreateType::TimerService => ".service".len(),
            UnitCreateType::Unknown => 0,
        }
    }
}

impl From<&glib::Variant> for UnitCreateType {
    fn from(value: &glib::Variant) -> Self {
        match value.get::<String>().as_deref() {
            Some("service") => UnitCreateType::Service,
            Some("timer") => UnitCreateType::Timer,
            Some("timer_service") => UnitCreateType::TimerService,
            other => {
                warn!("Unkown type {:?}", other);
                UnitCreateType::Unknown
            }
        }
    }
}
