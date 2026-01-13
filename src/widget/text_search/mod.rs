use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::pgettext;
use glib::object::{CastNone, ObjectExt};
use gtk::{glib, prelude::TextViewExt};
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

fn set_menu_item(text_view: &gtk::TextView, action_name_base: &str) {
    let menu = gio::Menu::new();

    // Find in text Menu
    let menu_label = pgettext("text_find", "Find Text");

    let mut action_name = String::from("win.");
    action_name.push_str(action_name_base);

    menu.append(Some(&menu_label), Some(&action_name));

    text_view.set_extra_menu(Some(&menu));
}

pub fn text_search_construct(
    text_view: &gtk::TextView,
    text_search_bar: &gtk::SearchBar,
    find_text_button: &gtk::ToggleButton,
    action_name_base: &str,
) {
    set_menu_item(text_view, action_name_base);

    let text_search_bar_content = TextSearchBar::new(text_view);
    set_menu_item(text_view, action_name_base);

    text_search_bar.set_child(Some(&text_search_bar_content));

    find_text_button
        .bind_property("active", text_search_bar, "search-mode-enabled")
        .bidirectional()
        .build();

    text_search_bar.connect_search_mode_enabled_notify(|search_bar| {
        if let Some(text_search_bar) = search_bar.child().and_downcast_ref::<TextSearchBar>() {
            if search_bar.is_search_mode() {
                text_search_bar.find_text();
            } else {
                text_search_bar.clear_tags();
            }
        };
    });
}

pub fn on_new_text(search_bar: &gtk::SearchBar) {
    if !search_bar.is_search_mode() {
        return;
    }

    if let Some(text_search_bar) = search_bar.child().and_downcast_ref::<TextSearchBar>() {
        text_search_bar.find_text();
    }
}

pub fn update_text_view(
    text_search_bar: &gtk::SearchBar,
    text_view: &gtk::TextView,
    action_name_base: &str,
) {
    set_menu_item(text_view, action_name_base);
    if let Some(search_bar) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search_bar.imp().set_text_view(text_view);
    }
}

pub fn new_added_text(
    text_search_bar: &gtk::SearchBar,
    buff: &gtk::TextBuffer,
    start_iter: gtk::TextIter,
    end_iter: gtk::TextIter,
) {
    if let Some(search_bar) = text_search_bar.child().and_downcast_ref::<TextSearchBar>() {
        search_bar.imp().new_added_text(buff, start_iter, end_iter);
    }
}
