use crate::{
    gtk::prelude::*,
    systemd::data::UnitInfo,
    widget::unit_list::{COL_ID_UNIT, imp::column_factories::*, menus::create_col_menu},
};
use gettextrs::pgettext;

pub fn construct_column(
    list_store: gio::ListStore,
    display_color: bool,
) -> (
    gtk::ColumnView,
    gtk::SingleSelection,
    gtk::FilterListModel,
    gtk::SortListModel,
) {
    let sort_list_model = gtk::SortListModel::new(Some(list_store), None::<gtk::Sorter>);
    let filter_list_model =
        gtk::FilterListModel::new(Some(sort_list_model.clone()), None::<gtk::Filter>);
    let selection_model = gtk::SingleSelection::builder()
        .model(&filter_list_model)
        .autoselect(false)
        .build();
    let column_view = gtk::ColumnView::new(Some(selection_model.clone()));

    set_base_columns(&column_view, display_color);
    let sorter = column_view.sorter();
    sort_list_model.set_sorter(sorter.as_ref());

    (
        column_view,
        selection_model,
        filter_list_model,
        sort_list_model,
    )
}

macro_rules! compare_units {
    ($unit1:expr, $unit2:expr, $func:ident) => {{
        $unit1.$func().cmp(&$unit2.$func()).into()
    }};

    ($unit1:expr, $unit2:expr, $func:ident, $($funcx:ident),+) => {{
        let ordering = $unit1.$func().cmp(&$unit2.$func());
        if ordering != core::cmp::Ordering::Equal {
            return ordering.into();
        }

        compare_units!($unit1, $unit2, $($funcx),+)
    }};
}

macro_rules! column_filter_lambda {
 ($($func:ident),+) => {{
    |obj1: &gtk::glib::Object, obj2: &gtk::glib::Object| {
            let unit1 = obj1
                .downcast_ref::<UnitInfo>()
                .expect("Needs to be UnitInfo");
            let unit2 = obj2
                .downcast_ref::<UnitInfo>()
                .expect("Needs to be UnitInfo");

            compare_units!(unit1, unit2, $($func),+)
        }
 }}
}

pub(crate) use column_filter_lambda;

macro_rules! create_column_filter {
    ($($func:ident),+) => {{
        gtk::CustomSorter::new(column_filter_lambda!( $($func),+))
    }};
}

const GETTEXT_CONTEXT: &str = "list column";
fn set_base_columns(column_view: &gtk::ColumnView, display_color: bool) {
    let id = COL_ID_UNIT;
    let sorter = create_column_filter!(primary, dbus_level);
    let column_menu = create_col_menu(id, false);
    let factory = fac_unit_name(display_color);
    let unit_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(150)
        .title(pgettext(GETTEXT_CONTEXT, "Unit"))
        .build();

    let id = "sysdm-type";
    let sorter = create_column_filter!(unit_type);
    let column_menu = create_col_menu(id, false);
    let factory = fac_unit_type(display_color);
    let type_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(82)
        .title(pgettext(GETTEXT_CONTEXT, "Type"))
        .build();

    let id = "sysdm-bus";
    let sorter = create_column_filter!(dbus_level);
    let column_menu = create_col_menu(id, false);
    let factory = fac_bus(display_color);
    let bus_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(61)
        .title(pgettext(GETTEXT_CONTEXT, "Bus"))
        .build();

    let id = "sysdm-state";
    let sorter = create_column_filter!(enable_status);
    let column_menu = create_col_menu(id, false);
    let factory = fac_enable_status(display_color);
    let state_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(80)
        .title(pgettext(GETTEXT_CONTEXT, "State"))
        .build();

    let id = "sysdm-preset";
    let sorter = create_column_filter!(preset);
    let column_menu = create_col_menu(id, false);
    let factory = fac_preset(display_color);
    let preset_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(70)
        .title(pgettext(GETTEXT_CONTEXT, "Preset"))
        .build();

    let id = "sysdm-load";
    let sorter = create_column_filter!(load_state);
    let column_menu = create_col_menu(id, false);
    let factory = fac_load_state(display_color);
    let load_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(80)
        .title(pgettext(GETTEXT_CONTEXT, "Load"))
        .build();

    let id = "sysdm-active";
    let sorter = create_column_filter!(active_state);
    let column_menu = create_col_menu(id, false);
    let factory = fac_active(display_color);
    let active_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(62)
        .title(pgettext(GETTEXT_CONTEXT, "Active"))
        .build();

    let id = "sysdm-sub";
    let sorter = create_column_filter!(sub_state);
    let column_menu = create_col_menu(id, false);
    let factory = fac_sub_state(display_color);
    let sub_col = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(71)
        .title(pgettext(GETTEXT_CONTEXT, "Sub"))
        .build();

    let id = "sysdm-description";
    let sorter = create_column_filter!(description);
    let column_menu = create_col_menu(id, false);
    let factory = fac_descrition(display_color);
    let sub_description = gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .expand(true)
        .title(pgettext(GETTEXT_CONTEXT, "Description"))
        .build();

    column_view.append_column(&unit_col);
    column_view.append_column(&type_col);
    column_view.append_column(&bus_col);
    column_view.append_column(&state_col);
    column_view.append_column(&preset_col);
    column_view.append_column(&load_col);
    column_view.append_column(&active_col);
    column_view.append_column(&sub_col);
    column_view.append_column(&sub_description);
}
