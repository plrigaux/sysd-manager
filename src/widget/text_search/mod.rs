use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;
mod imp;

glib::wrapper! {
    pub struct TextSearchBar(ObjectSubclass<imp::TextSearchBarImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl TextSearchBar {
    pub fn new(text_view: &gtk::TextView) -> TextSearchBar {
        let obj: TextSearchBar = glib::Object::new();

        obj.imp().set_text_view(text_view);

        obj
    }

    pub fn grab_focus_on_search_entry(&self) {
        self.imp().grab_focus_on_search_entry();
    }
}
