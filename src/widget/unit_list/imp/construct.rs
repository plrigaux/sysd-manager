use crate::{
    gtk::prelude::*,
    systemd::data::UnitInfo,
    widget::{
        unit_list::{COL_ID_UNIT, imp::column_factories::*, menus::create_col_menu},
        unit_properties_selector::{
            data_selection::UnitPropertySelection, save::load_column_config,
        },
    },
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
    bool,
) {
    let sort_list_model = gtk::SortListModel::new(Some(list_store), None::<gtk::Sorter>);
    let filter_list_model =
        gtk::FilterListModel::new(Some(sort_list_model.clone()), None::<gtk::Filter>);
    let selection_model = gtk::SingleSelection::builder()
        .model(&filter_list_model)
        .autoselect(false)
        .build();
    let column_view = gtk::ColumnView::new(Some(selection_model.clone()));

    let (base_columns, generated) = if let Some(col) = load_column_config() {
        let mut list = Vec::with_capacity(col.column.len());
        for unit_column_config in col.column {
            let id = unit_column_config.id;
            let sorter = get_sorter_by_id(&id);
            let factory = get_factory_by_id(&id, display_color);

            let column = gtk::ColumnViewColumn::builder()
                .id(&id)
                .fixed_width(unit_column_config.fixed_width)
                .expand(unit_column_config.expands)
                .resizable(unit_column_config.resizable)
                .visible(unit_column_config.visible)
                .build();

            column.set_title(unit_column_config.title.as_deref());
            let column_menu = create_col_menu(&id, false);
            column.set_header_menu(Some(&column_menu));
            column.set_factory(factory.as_ref());
            column.set_sorter(sorter.as_ref());

            list.push(column);
        }
        (list, false)
    } else {
        (generate_default_columns(display_color), true)
    };

    for col in base_columns {
        column_view.append_column(&col);
    }

    let sorter = column_view.sorter();
    sort_list_model.set_sorter(sorter.as_ref());

    (
        column_view,
        selection_model,
        filter_list_model,
        sort_list_model,
        generated,
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
use log::warn;

macro_rules! create_column_filter {
    ($($func:ident),+) => {{
        gtk::CustomSorter::new(column_filter_lambda!( $($func),+))
    }};
}

pub fn default_column_definition_list(display_color: bool) -> Vec<UnitPropertySelection> {
    generate_default_columns(display_color)
        .into_iter()
        .map(UnitPropertySelection::from_column_view_column)
        .collect()
}

pub fn get_sorter_by_id(id: &str) -> Option<gtk::CustomSorter> {
    match id {
        COL_ID_UNIT => Some(create_column_filter!(primary, dbus_level)),
        "sysdm-type" => Some(create_column_filter!(unit_type)),
        "sysdm-bus" => Some(create_column_filter!(dbus_level)),
        "sysdm-state" => Some(create_column_filter!(enable_status)),
        "sysdm-preset" => Some(create_column_filter!(preset)),
        "sysdm-load" => Some(create_column_filter!(load_state)),
        "sysdm-active" => Some(create_column_filter!(active_state)),
        "sysdm-sub" => Some(create_column_filter!(sub_state)),
        "sysdm-description" => Some(create_column_filter!(description)),

        _ => {
            warn!("What to do. Id {id} not handle with sorter");
            None
        }
    }
}

const GETTEXT_CONTEXT: &str = "list column";
fn generate_default_columns(display_color: bool) -> Vec<gtk::ColumnViewColumn> {
    let mut columns = vec![];

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
    columns.push(unit_col);

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
    columns.push(type_col);

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
    columns.push(bus_col);

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
    columns.push(state_col);

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
    columns.push(preset_col);

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
    columns.push(load_col);

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
    columns.push(active_col);

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
    columns.push(sub_col);

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
    columns.push(sub_description);

    columns
}
