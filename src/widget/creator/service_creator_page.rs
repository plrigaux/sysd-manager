mod imp;
pub mod standard_output;

use crate::widget::creator::{
    PageType, UnitCreatorWindow, unit_file_creator_page::UnitFileCreatorPage,
};
use adw::prelude::NavigationPageExt;
use glib::{WeakRef, subclass::types::ObjectSubclassIsExt};
use gtk::glib::{self};

glib::wrapper! {

    pub struct ServiceCreatorPage(ObjectSubclass<imp::ServiceCreatorPageImp>)
    @extends adw::NavigationPage,  gtk::Widget,
    @implements gtk::Accessible,  gtk::Buildable,  gtk::ConstraintTarget ;
}

impl ServiceCreatorPage {
    pub fn new(window: WeakRef<UnitCreatorWindow>, page: PageType) -> Self {
        let obj: ServiceCreatorPage = glib::Object::new();
        obj.set_tag(Some(page.id()));
        let _ = obj.imp().window.set(window);
        // obj.imp().update_from_unit_info();
        obj
    }

    pub fn update_view(&self, page: &UnitFileCreatorPage) {
        self.imp().update_view(page);
    }

    pub fn update_from_file_content(&self, content: &str) {
        self.imp().update_from_file_content(content);
    }

    pub fn file_content(&self) -> String {
        self.imp().file_content()
    }

    pub fn update_from_unit_info(&self) {
        self.imp().update_from_unit_info();
    }
}

pub const ENVIRONMENT: &str = "Environment";
