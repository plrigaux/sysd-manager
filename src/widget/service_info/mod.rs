use crate::{gtk::glib, systemd::data::UnitInfo};
use crate::gtk::subclass::prelude::*;
mod imp;

glib::wrapper! {
    pub struct ServiceStatus(ObjectSubclass<imp::ServiceStatusImp>)
        @extends gtk::Grid, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl ServiceStatus {
    pub fn new() -> Self {
        let obj: ServiceStatus = glib::Object::new();
        obj
    }

    pub fn fill_data(&self, unit : &UnitInfo) {
        self.imp().fill_data(unit)
    }
}
