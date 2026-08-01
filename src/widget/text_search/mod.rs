use crate::{
    consts::{ACTION_FIND_IN_TEXT_TOGGLE, SETTING_FIND_IN_TEXT_OPEN},
    widget,
};
use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::pgettext;
use glib::object::CastNone;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct TextSearchEntry(ObjectSubclass<imp::TextSearchEntryImp>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl TextSearchEntry {
    pub fn new(text_view: &gtk::TextView) -> TextSearchEntry {
        let obj: TextSearchEntry = glib::Object::new();

        obj.imp().set_text_view(text_view);

        obj
    }

    pub fn set_text_view(&self, text_view: &gtk::TextView) {
        self.imp().set_text_view(text_view);
    }

    pub fn grab_focus_on_search_entry(&self) {
        widget::grab_focus_on_search_entry(&self.imp().search_entry);
    }

    pub fn clear_tags(&self) {
        self.imp().clear_tags();
    }

    pub fn find_text(&self) {
        self.imp().new_find_in_text();
    }

    pub fn new_added_text(
        &self,
        buff: &gtk::TextBuffer,
        start_iter: gtk::TextIter,
        end_iter: gtk::TextIter,
    ) {
        self.imp().new_added_text(buff, start_iter, end_iter);
    }
}

pub fn create_menu_item(menu: &gio::Menu) {
    // Open in text Menu item
    let menu_label = pgettext("text_find", "Toggle Find in Text");
    let item = gio::MenuItem::new(Some(&menu_label), Some(SETTING_FIND_IN_TEXT_OPEN));

    menu.append_item(&item);

    let menu_label = pgettext("text_find", "Find in Text");
    let item = gio::MenuItem::new(Some(&menu_label), Some(ACTION_FIND_IN_TEXT_TOGGLE));

    menu.append_item(&item);
}

pub fn on_new_text(search_bar: &gtk::SearchBar) {
    if !search_bar.is_search_mode() {
        return;
    }

    if let Some(text_search_bar) = search_bar.child().and_downcast_ref::<TextSearchEntry>() {
        text_search_bar.find_text();
    }
}

pub fn focus_on_text_entry(text_search_bar: &gtk::SearchBar) {
    if let Some(search) = text_search_bar
        .child()
        .and_downcast_ref::<TextSearchEntry>()
    {
        search.grab_focus_on_search_entry();
    }
}
