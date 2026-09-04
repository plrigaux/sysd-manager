use gettextrs::pgettext;
use gtk::glib::variant::ToVariant;

use crate::{
    consts::{
        ACTION_APP_PROPERTIES_SELECTOR, ACTION_WIN_HIDE_UNIT_COL,
        ACTION_WIN_KEY_PREF_UNIT_LIST_ACTIVE_STAUTUS_AS_ICON, ACTION_WIN_RESET_ALL_COLUMNS,
        NS_ACTION_UNIT_LIST_FILTER, NS_ACTION_UNIT_LIST_FILTER_CLEAR,
    },
    widget::unit_list::column::SysdColumn,
};

pub fn create_col_menu(key: &SysdColumn) -> gio::MenuModel {
    let menu = gio::Menu::new();

    let variant = key.id().to_variant();
    append_item_variant(
        &menu,
        //column header menu
        &pgettext("menu", "Hide this Column"),
        ACTION_WIN_HIDE_UNIT_COL,
        &variant,
    );

    append_item_variant(
        &menu,
        //column header menu
        &pgettext("menu", "Configure columns"),
        ACTION_APP_PROPERTIES_SELECTOR,
        &variant,
    );

    if &SysdColumn::Active == key {
        let item = gio::MenuItem::new(
            Some(&pgettext("menu", "Display as Icon")),
            Some(ACTION_WIN_KEY_PREF_UNIT_LIST_ACTIVE_STAUTUS_AS_ICON),
        );
        menu.append_item(&item);
    }

    let item = gio::MenuItem::new(
        Some(&pgettext("menu", "Reset all columns")),
        Some(ACTION_WIN_RESET_ALL_COLUMNS),
    );
    menu.append_item(&item);

    if key.is_custom() {
        let sub_menu = gio::Menu::new();

        append_item_variant(
            &sub_menu,
            //column header menu
            &pgettext("menu", "Configure Filters"),
            NS_ACTION_UNIT_LIST_FILTER,
            &variant,
        );

        append_item_variant(
            &sub_menu,
            //column header menu
            &pgettext("menu", "Clear Column Filter"),
            NS_ACTION_UNIT_LIST_FILTER_CLEAR,
            &variant,
        );

        //column header menu section
        menu.append_section(Some(&pgettext("menu", "Filtering")), &sub_menu);
    }
    menu.freeze();

    menu.into()
}

fn append_item_variant(menu: &gio::Menu, title: &str, action: &str, target_value: &glib::Variant) {
    let item: gio::MenuItem = gio::MenuItem::new(Some(title), None);
    item.set_action_and_target_value(Some(action), Some(target_value));
    menu.append_item(&item);
}
