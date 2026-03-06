use gettextrs::pgettext;
use gtk::glib::variant::ToVariant;

use crate::{
    consts::{
        ACTION_WIN_COL_RESIZE, ACTION_WIN_HIDE_UNIT_COL, APP_ACTION_PROPERTIES_SELECTOR,
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
        APP_ACTION_PROPERTIES_SELECTOR,
        &variant,
    );

    append_item_variant(
        &menu,
        //column header menu
        &pgettext("menu", "Col Resize"),
        ACTION_WIN_COL_RESIZE,
        &variant,
    );

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
