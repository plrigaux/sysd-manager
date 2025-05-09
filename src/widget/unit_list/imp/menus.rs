use gtk::glib::variant::ToVariant;

use crate::{consts::NS_ACTION_UNIT_LIST_FILTER_CLEAR, widget::preferences::data::COL_SHOW_PREFIX};

pub(super) fn create_col_menu(key: &str) -> gio::MenuModel {
    let menu = gio::Menu::new();

    append_item_variant(
        &menu,
        "Hide this Column",
        "win.hide_unit_col",
        &format!("{}{key}", COL_SHOW_PREFIX),
    );

    let sub_menu = gio::Menu::new();
    sub_menu.append(Some("Unit"), Some("win.col-show-unit"));
    sub_menu.append(Some("Type"), Some("win.col-show-type"));
    sub_menu.append(Some("Bus"), Some("win.col-show-bus"));
    sub_menu.append(Some("State"), Some("win.col-show-state"));
    sub_menu.append(Some("Preset"), Some("win.col-show-preset"));
    sub_menu.append(Some("Load"), Some("win.col-show-load"));
    sub_menu.append(Some("Active"), Some("win.col-show-active"));
    sub_menu.append(Some("Sub"), Some("win.col-show-sub"));
    sub_menu.append(Some("Description"), Some("win.col-show-description"));
    menu.append_submenu(Some("Show columns"), &sub_menu);

    let sub_menu = gio::Menu::new();

    append_item_variant(&sub_menu, "Filter", "win.unit_list_filter", key);
    append_item_variant(
        &sub_menu,
        "Clear Filters",
        NS_ACTION_UNIT_LIST_FILTER_CLEAR,
        key,
    );

    menu.append_section(Some("Filterring"), &sub_menu);
    menu.freeze();

    menu.into()
}

fn append_item_variant(menu: &gio::Menu, title: &str, action: &str, target_value: &str) {
    let item: gio::MenuItem = gio::MenuItem::new(Some(title), None);
    let target_value = target_value.to_variant();
    item.set_action_and_target_value(Some(action), Some(&target_value));
    menu.append_item(&item);
}
