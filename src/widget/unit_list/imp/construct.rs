use std::collections::HashMap;

use crate::{
    consts::{
        SOCKET_LISTEN_COL, SOCKET_LISTEN_TYPE, TIME_LAST_TRIGGER_USEC,
        TIME_NEXT_ELAPSE_USEC_MONOTONIC, TIME_NEXT_ELAPSE_USEC_REALTIME, TIMER_TIME_LAST,
        TIMER_TIME_LEFT, TIMER_TIME_NEXT, TIMER_TIME_PASSED,
    },
    gtk::prelude::*,
    systemd::data::UnitInfo,
    widget::{
        unit_list::{
            COL_ID_UNIT, COL_ID_UNIT_FULL, CustomPropertyId, UnitListView,
            imp::{
                column_factories::{self, *},
                construct,
            },
            menus::create_col_menu,
        },
        unit_properties_selector::{
            data_selection::UnitPropertySelection,
            save::{self, SortType, UnitColumn},
        },
    },
};
use gettextrs::pgettext;
use log::warn;
use zvariant::Value;

pub fn construct_column_view(
    display_color: bool,
    view: UnitListView,
) -> Vec<UnitPropertySelection> {
    let list = build_from_load(display_color, view);

    let default_column_set = match view {
        UnitListView::Defaut => default_column_definition_list(display_color),
        UnitListView::LoadedUnit => generate_loaded_units_columns(display_color),
        UnitListView::UnitFiles => generate_unit_files_columns(display_color),
        UnitListView::Timers => generate_timers_columns(display_color),
        UnitListView::Sockets => generate_sockets_columns(display_color),
        UnitListView::Custom => {
            if list.is_empty() {
                return default_column_definition_list(display_color);
            }
            return list;
        }
    };

    let mut dict: HashMap<_, _> = list
        .into_iter()
        .filter_map(|c| c.id().map(|id| (id, c)))
        .collect();

    let mut out = Vec::with_capacity(default_column_set.len());
    for (id, default_up) in default_column_set
        .into_iter()
        .filter_map(|c| c.id().map(|id| (id, c)))
    {
        let unit_prop = if let Some(loaded_up) = dict.remove(&id) {
            loaded_up.set_sort(default_up.sort());
            loaded_up
        } else {
            default_up
        };

        unit_prop.column().set_expand(false);
        out.push(unit_prop);
    }
    out
}

fn generate_sockets_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(unit_col));

    let col = create_unit_active_status_columun(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(col));

    let col = create_socket_listen_type_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_socket_listen_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_col_activates();
    columns.push(UnitPropertySelection::from_column_config(col));

    columns
}

fn generate_timers_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(unit_col));

    let col = create_time_next_time();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_time_next_delay();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_time_last();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_time_passed();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_col_activates();
    columns.push(UnitPropertySelection::from_column_config(col));

    columns
}

fn generate_unit_files_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_name_column(display_color);

    columns.push(UnitPropertySelection::from_column_view_column(unit_col));

    let type_col = create_unit_type_column(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(type_col));

    let state_col = create_unit_file_state(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(state_col));

    let preset_col = create_unit_file_preset_column(display_color);
    columns.push(UnitPropertySelection::from_column_view_column(preset_col));

    columns
}

pub fn build_from_load(display_color: bool, view: UnitListView) -> Vec<UnitPropertySelection> {
    let Some(saved_config) = save::load_column_config(view) else {
        return vec![];
    };

    let mut list = Vec::with_capacity(saved_config.columns.len());
    for unit_column_config in saved_config.columns {
        let id = unit_column_config.id.clone();
        let prop_selection = UnitPropertySelection::from_column_config(unit_column_config);

        let column_menu = create_col_menu(&id, prop_selection.is_custom());
        let column = prop_selection.column();
        column.set_header_menu(Some(&column_menu));

        let prop_type = prop_selection.prop_type();

        construct::set_column_factory_and_sorter(&column, display_color, prop_type.as_deref());

        list.push(prop_selection);
    }
    list
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

#[macro_export]
macro_rules! column_sorter_lambda {
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

macro_rules! create_column_filter {
    ($($func:ident),+) => {{
        gtk::CustomSorter::new(column_sorter_lambda!( $($func),+))
    }};
}

pub fn default_column_definition_list(display_color: bool) -> Vec<UnitPropertySelection> {
    generate_default_columns(display_color)
        .into_iter()
        .map(UnitPropertySelection::from_column_view_column)
        .collect()
}

pub fn set_column_factory_and_sorter(
    column: &gtk::ColumnViewColumn,
    display_color: bool,
    prop_type: Option<&str>,
) {
    let Some(id) = column.id() else {
        warn!("No column id");
        return;
    };

    //identify custom properties
    let custom_id = CustomPropertyId::from_str(id.as_str());

    //force data display
    let factory = column_factories::get_factory_by_id(&custom_id, display_color, prop_type);
    column.set_factory(factory.as_ref());

    let sorter = get_sorter_by_id(custom_id, prop_type);
    column.set_sorter(sorter.as_ref());
}

pub fn get_sorter_by_id(
    id: CustomPropertyId,
    prop_type: Option<&str>,
) -> Option<gtk::CustomSorter> {
    match id.prop {
        COL_ID_UNIT => Some(create_column_filter!(primary, dbus_level)),
        COL_ID_UNIT_FULL => Some(create_column_filter!(primary, dbus_level)),
        "sysdm-type" => Some(create_column_filter!(unit_type)),
        "sysdm-bus" => Some(create_column_filter!(dbus_level)),
        "sysdm-state" => Some(create_column_filter!(enable_status)),
        "sysdm-preset" => Some(create_column_filter!(preset)),
        "sysdm-load" => Some(create_column_filter!(load_state)),
        "sysdm-active" => Some(create_column_filter!(active_state)),
        "sysdm-sub" => Some(create_column_filter!(sub_state)),
        "sysdm-description" => Some(create_column_filter!(description)),
        TIMER_TIME_NEXT | TIMER_TIME_LEFT => create_next_elapse_column_filter(),
        TIMER_TIME_PASSED | TIMER_TIME_LAST => {
            create_not_so_custom_property_colum_sorter(TIME_LAST_TRIGGER_USEC, "t")
        }

        _ => create_custom_property_column_sorter(id, prop_type),
    }
}

fn create_custom_property_column_sorter(
    id: CustomPropertyId,
    prop_type: Option<&str>,
) -> Option<gtk::CustomSorter> {
    let key = id.generate_quark();

    let Some(prop_type) = prop_type else {
        warn!("column sorter without prop_type ");
        return None;
    };

    create_column_sorter(key, prop_type)
}

fn create_not_so_custom_property_colum_sorter(
    key_id: &str,
    col_type: &str,
) -> Option<gtk::CustomSorter> {
    let key = glib::Quark::from_str(key_id);
    create_column_sorter(key, col_type)
}

fn create_column_sorter(key: glib::Quark, prop_type: &str) -> Option<gtk::CustomSorter> {
    let sort_func = match prop_type {
        "b" => custom_property_comapre::<bool>,
        "n" => custom_property_comapre::<i16>,
        "q" => custom_property_comapre::<u16>,
        "i" => custom_property_comapre::<i32>,
        "u" => custom_property_comapre::<u32>,
        "x" => custom_property_comapre::<i64>,
        "t" => custom_property_comapre::<u64>,
        "v" => custom_property_comapre::<Value>,
        "s" => custom_property_comapre::<String>,
        _ => custom_property_comapre::<String>,
    };

    let sorter = gtk::CustomSorter::new(move |o1, o2| sort_func(o1, o2, key));

    Some(sorter)
}

fn create_next_elapse_column_filter() -> Option<gtk::CustomSorter> {
    let next_elapse_realtime_key = glib::Quark::from_str(TIME_NEXT_ELAPSE_USEC_REALTIME);
    let next_elapse_monotonic_key = glib::Quark::from_str(TIME_NEXT_ELAPSE_USEC_MONOTONIC);

    let sorter = gtk::CustomSorter::new(move |o1, o2| {
        let next_elapse1 =
            calculate_next_elapse(next_elapse_realtime_key, next_elapse_monotonic_key, o1);
        let next_elapse2 =
            calculate_next_elapse(next_elapse_realtime_key, next_elapse_monotonic_key, o2);

        next_elapse1.cmp(&next_elapse2).into()
    });

    Some(sorter)
}

fn custom_property_comapre<T>(
    object1: &glib::Object,
    object2: &glib::Object,
    key: glib::Quark,
) -> gtk::Ordering
where
    T: Ord + 'static,
{
    let v1 = unsafe { object1.qdata::<T>(key).map(|value_ptr| value_ptr.as_ref()) };
    let v2 = unsafe { object2.qdata::<T>(key).map(|value_ptr| value_ptr.as_ref()) };

    v1.into_iter().cmp(v2).into()
}

const SYSDM_STATE: &str = "sysdm-state";
const SYSDM_PRESET: &str = "sysdm-preset";

fn generate_default_columns(display_color: bool) -> Vec<gtk::ColumnViewColumn> {
    let mut columns = vec![];

    let unit_col = create_unit_display_name_column(display_color);
    columns.push(unit_col);

    let type_col = create_unit_type_column(display_color);
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
        .title(pgettext("list column", "Bus"))
        .build();
    columns.push(bus_col);

    let state_col = create_unit_file_state(display_color);
    columns.push(state_col);

    let preset_col = create_unit_file_preset_column(display_color);
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
        .title(pgettext("list column", "Load"))
        .build();
    columns.push(load_col);

    let active_col = create_unit_active_status_columun(display_color);
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
        .title(pgettext("list column", "Sub"))
        .build();
    columns.push(sub_col);

    let sub_description = create_unit_description_column(display_color);
    columns.push(sub_description);

    columns
}

fn create_unit_file_preset_column(display_color: bool) -> gtk::ColumnViewColumn {
    let id = SYSDM_PRESET;
    let sorter = create_column_filter!(preset);
    let column_menu = create_col_menu(id, false);
    let factory = fac_preset(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(70)
        .title(pgettext("list column", "Preset"))
        .build()
}

fn create_unit_file_state(display_color: bool) -> gtk::ColumnViewColumn {
    let id = SYSDM_STATE;
    let sorter = create_column_filter!(enable_status);
    let column_menu = create_col_menu(id, false);
    let factory = fac_enable_status(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(80)
        .title(pgettext("list column", "State"))
        .build()
}

fn create_unit_active_status_columun(display_color: bool) -> gtk::ColumnViewColumn {
    let id = "sysdm-active";
    let sorter = create_column_filter!(active_state);
    let column_menu = create_col_menu(id, false);
    let factory = fac_active(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(62)
        .title(pgettext("list column", "Active"))
        .build()
}

fn create_unit_description_column(display_color: bool) -> gtk::ColumnViewColumn {
    let id = "sysdm-description";
    let sorter = create_column_filter!(description);
    let column_menu = create_col_menu(id, false);
    let factory = fac_descrition(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .expand(true)
        .title(pgettext("list column", "Description"))
        .build()
}

fn create_unit_display_name_column(display_color: bool) -> gtk::ColumnViewColumn {
    let id = COL_ID_UNIT;
    let sorter = create_column_filter!(primary, dbus_level);
    let column_menu = create_col_menu(id, false);
    let factory = fac_unit_name(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(150)
        .title(pgettext("list column", "Unit"))
        .build()
}

fn create_unit_display_full_name_column(display_color: bool) -> gtk::ColumnViewColumn {
    let id = COL_ID_UNIT_FULL;
    let sorter = create_column_filter!(primary, dbus_level);
    let column_menu = create_col_menu(id, false);
    let factory = fac_unit_name(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(150)
        .title(pgettext("list column", "Unit"))
        .build()
}

fn create_unit_type_column(display_color: bool) -> gtk::ColumnViewColumn {
    let id = "sysdm-type";
    let sorter = create_column_filter!(unit_type);
    let column_menu = create_col_menu(id, false);
    let factory = fac_unit_type(display_color);

    gtk::ColumnViewColumn::builder()
        .id(id)
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(82)
        .title(pgettext("list column", "Type"))
        .build()
}

fn create_socket_listen_type_column() -> UnitColumn {
    let mut unit_column = UnitColumn::new(SOCKET_LISTEN_TYPE, "a(ss)");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Listen Type"));
    unit_column.fixed_width = 120;

    unit_column
}

fn create_socket_listen_column() -> UnitColumn {
    let mut unit_column = UnitColumn::new(SOCKET_LISTEN_COL, "a(ss)");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Listen"));
    unit_column.fixed_width = 80;

    unit_column
}

fn create_time_next_time() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_NEXT, "t");
    unit_column.resizable = true;
    //Timer
    unit_column.title = Some(pgettext("list column", "Next"));
    unit_column.fixed_width = 120;
    unit_column.sort = Some(SortType::Asc);
    unit_column
}

fn create_time_next_delay() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_LEFT, "t");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Left"));
    unit_column.fixed_width = 120;

    unit_column
}

fn create_time_last() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_LAST, "t");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Last"));
    unit_column.fixed_width = 120;
    unit_column
}

fn create_time_passed() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_PASSED, "t");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Passed"));
    unit_column.fixed_width = 120;
    unit_column
}

fn create_col_activates() -> UnitColumn {
    let id = "unit@Triggers";

    let mut unit_column = UnitColumn::new(id, "as");
    unit_column.resizable = true;
    unit_column.title = Some(pgettext("list column", "Activates"));
    unit_column.fixed_width = 120;

    unit_column
}

fn generate_loaded_units_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    generate_default_columns(display_color)
        .into_iter()
        .filter(|c| {
            c.id().map(|s| s.as_str() != SYSDM_STATE).unwrap_or(true)
                && c.id().map(|s| s.as_str() != SYSDM_PRESET).unwrap_or(true)
        })
        .map(UnitPropertySelection::from_column_view_column)
        .collect()
}
