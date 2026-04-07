use crate::{
    consts::{
        PATH_CONDITION_COL, PATH_PATH_COL, SOCKET_LISTEN_COL, SOCKET_LISTEN_TYPE,
        SYSD_SOCKET_LISTEN, TIME_LAST_TRIGGER_USEC, TIMER_TIME_LAST, TIMER_TIME_LEFT,
        TIMER_TIME_NEXT, TIMER_TIME_PASSED,
    },
    extract_listen, extract_tuple_idx,
    gtk::prelude::*,
    systemd::data::UnitInfo,
    widget::{
        unit_list::{
            UnitCuratedList,
            column::SysdColumn,
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
use std::{cell::OnceCell, collections::HashMap, rc::Rc};
use systemd::{enums::UnitType, runtime, socket_unit::SocketUnitInfo};
use tracing::{info, warn};
use zvariant::Value;

pub fn construct_column_view(
    display_color: bool,
    view: UnitCuratedList,
    include_unit_files: bool,
) -> Vec<UnitPropertySelection> {
    let list = build_from_load(display_color, view);

    let default_column_set = match view {
        UnitCuratedList::Defaut => default_column_definition_list(display_color),
        UnitCuratedList::LoadedUnit => generate_loaded_units_columns(display_color),
        UnitCuratedList::UnitFiles => generate_unit_files_columns(display_color),
        UnitCuratedList::Timers => generate_timers_columns(display_color, include_unit_files),
        UnitCuratedList::Sockets => generate_sockets_columns(display_color, include_unit_files),
        UnitCuratedList::Path => generate_paths_columns(display_color, include_unit_files),
        UnitCuratedList::Automount => {
            generate_automounts_columns(display_color, include_unit_files)
        }
        UnitCuratedList::Custom => {
            if list.is_empty() {
                return default_column_definition_list(display_color);
            }
            return list;
        }
        UnitCuratedList::Favorite => default_column_definition_list(display_color),
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

fn generate_automounts_columns(
    display_color: bool,
    include_unit_files: bool,
) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(unit_col);

    if include_unit_files {
        let col = create_unit_file_state(display_color);
        columns.push(col);
    }

    let sysd_col = SysdColumn::fill_custom(UnitType::Mount, "Where", "s");

    let unit_column = UnitColumn {
        resizable: true,
        //Automounts list column name
        title: Some(pgettext("list column", "Where")),
        fixed_width: 120,
        ..Default::default()
    };
    columns.push(UnitPropertySelection::from_column_config2(
        unit_column,
        sysd_col,
    ));

    let sysd_col = SysdColumn::AutomountWhat;
    let unit_column = UnitColumn {
        resizable: true,
        //Automounts list column name
        title: Some(pgettext("list column", "What")),
        fixed_width: 120,
        ..Default::default()
    };
    columns.push(UnitPropertySelection::from_column_config2(
        unit_column,
        sysd_col,
    ));

    let sysd_col = SysdColumn::AutomountMounted;
    let unit_column = UnitColumn {
        resizable: true,
        //Automounts list column name
        title: Some(pgettext("list column", "Mounted")),
        fixed_width: 120,
        ..Default::default()
    };
    columns.push(UnitPropertySelection::from_column_config2(
        unit_column,
        sysd_col,
    ));

    let col = SysdColumn::AutomountIdleTimeOut;
    let unit_column = UnitColumn {
        resizable: true,
        //Automounts list column name
        title: Some(pgettext("list column", "Idle Timeout")),
        fixed_width: 120,
        ..Default::default()
    };
    columns.push(UnitPropertySelection::from_column_config2(unit_column, col));

    columns
}

fn generate_sockets_columns(
    display_color: bool,
    include_unit_files: bool,
) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(unit_col);

    if include_unit_files {
        let col = create_unit_file_state(display_color);
        columns.push(col);
    }

    let col = create_unit_active_status_columun(display_color);
    columns.push(col);

    let col = create_socket_listen_type_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_socket_listen_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_col_activates();
    columns.push(UnitPropertySelection::from_column_config(col));

    columns
}

fn generate_timers_columns(
    display_color: bool,
    include_unit_files: bool,
) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(unit_col);

    if include_unit_files {
        let col = create_unit_file_state(display_color);
        columns.push(col);
    }

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

fn generate_paths_columns(
    display_color: bool,
    include_unit_files: bool,
) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_full_name_column(display_color);
    columns.push(unit_col);

    if include_unit_files {
        let col = create_unit_file_state(display_color);
        columns.push(col);
    }

    let col = create_path_paths_column();
    columns.push(col);

    let col = create_path_condition_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    let col = create_path_unit_column();
    columns.push(UnitPropertySelection::from_column_config(col));

    columns
}

fn create_path_condition_column() -> UnitColumn {
    let mut unit_column = UnitColumn::new(PATH_CONDITION_COL, "a(ss)");
    unit_column.resizable = true;
    //Path list column name
    unit_column.title = Some(pgettext("list column", "Condition"));
    unit_column.fixed_width = 320;

    unit_column
}

fn create_path_paths_column() -> UnitPropertySelection {
    let sysd_col = SysdColumn::Path;
    let unit_column = UnitColumn {
        resizable: true,
        //Path list column name
        title: Some(pgettext("list column", "Path")),
        fixed_width: 320,
        sort: Some(SortType::Asc),
        ..Default::default()
    };

    // let mut unit_column = UnitColumn::new(PATH_PATH_COL, "a(ss)");
    // unit_column.resizable = true;
    // //Path list column name
    // unit_column.title = Some(pgettext("list column", "Path"));
    // unit_column.fixed_width = 320;
    // unit_column.sort = Some(SortType::Asc);

    UnitPropertySelection::from_column_config2(unit_column, sysd_col)
}

fn create_path_unit_column() -> UnitColumn {
    let id = "path@Unit";

    let mut unit_column = UnitColumn::new(id, "s");
    unit_column.resizable = true;
    //Path list column name
    unit_column.title = Some(pgettext("list column", "Unit"));
    unit_column.fixed_width = 120;

    unit_column
}

fn generate_unit_files_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_name_column(display_color);

    columns.push(unit_col);

    let type_col = create_unit_type_column(display_color);
    columns.push(type_col);

    let state_col = create_unit_file_state(display_color);
    columns.push(state_col);

    let preset_col = create_unit_file_preset_column(display_color);
    columns.push(preset_col);

    columns
}

pub fn build_from_load(display_color: bool, view: UnitCuratedList) -> Vec<UnitPropertySelection> {
    let Some(saved_config) = save::load_column_config(view) else {
        return vec![];
    };

    let oc = OnceCell::new();
    let mut list = Vec::with_capacity(saved_config.columns.len());
    for unit_column_config in saved_config.columns {
        let id = match SysdColumn::verify(&unit_column_config) {
            Ok(sysd_column) => sysd_column,
            Err((_e, err_sc)) => {
                let m = oc.get_or_init(|| {
                    info!("Fetching Unit Properties for Sanitation purpose");
                    xxx()
                });
                if let Some((property_name, (interface, signature))) =
                    m.get_key_value(&unit_column_config.id)
                {
                    SysdColumn::new_from_props(property_name, interface, Some(signature.to_owned()))
                } else if let Some((_utype, prop)) = unit_column_config.id.split_once('@') {
                    if let Some((property_name, (interface, signature))) = m.get_key_value(prop) {
                        SysdColumn::new_from_props(
                            property_name,
                            interface,
                            Some(signature.to_owned()),
                        )
                    } else {
                        err_sc
                    }
                } else {
                    err_sc
                }
            }
        };

        // let id = unit_column_config.get_column();

        let prop_selection =
            UnitPropertySelection::from_column_config2(unit_column_config, id.clone());

        let column = prop_selection.column();

        construct::set_column_factory_and_sorter(&column, display_color, &id);

        list.push(prop_selection);
    }
    list
}

fn xxx() -> HashMap<String, (Rc<String>, String)> {
    let mut map = HashMap::new();
    for (interface, fetch_results) in runtime()
        .block_on(async move { systemd::fetch_unit_interface_properties().await })
        .inspect_err(|err| warn!("Fetch prop errors {err:?}"))
        .unwrap_or_default()
    {
        let interface = Rc::new(interface);
        for fetch_result in fetch_results {
            map.insert(
                fetch_result.name,
                (interface.clone(), fetch_result.signature),
            );
        }
    }
    map
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
}

pub fn set_column_factory_and_sorter(
    column: &gtk::ColumnViewColumn,
    display_color: bool,
    id: &SysdColumn,
) {
    //force data display
    let factory = column_factories::get_factory_by_id(id, display_color);
    column.set_factory(factory.as_ref());

    let sorter = get_sorter_by_id(id);
    column.set_sorter(sorter.as_ref());
}

pub fn get_sorter_by_id(id: &SysdColumn) -> Option<gtk::CustomSorter> {
    match id {
        SysdColumn::Name => Some(create_column_filter!(primary, dbus_level)),
        SysdColumn::FullName => Some(create_column_filter!(primary, dbus_level)),
        SysdColumn::Type => Some(create_column_filter!(unit_type)),
        SysdColumn::Bus => Some(create_column_filter!(dbus_level)),
        SysdColumn::State => Some(create_column_filter!(enable_status)),
        SysdColumn::Preset => Some(create_column_filter!(preset)),
        SysdColumn::Load => Some(create_column_filter!(load_state)),
        SysdColumn::Active => Some(create_column_filter!(active_state)),
        SysdColumn::SubState => Some(create_column_filter!(sub_state)),
        SysdColumn::Description => Some(create_column_filter!(description)),
        SysdColumn::TimerTimeNextElapseRT | SysdColumn::TimerTimeLeftElapseMono => {
            create_next_elapse_column_filter()
        }
        SysdColumn::TimerTimePassed | SysdColumn::TimerTimeLast => {
            create_not_so_custom_property_colum_sorter(TIME_LAST_TRIGGER_USEC, "t")
        }
        SysdColumn::SocketListen => create_socket_listen_type_colum_sorter(SYSD_SOCKET_LISTEN, 1),
        SysdColumn::SocketListenType => {
            create_socket_listen_type_colum_sorter(SYSD_SOCKET_LISTEN, 0)
        }
        SysdColumn::PathCondition => create_socket_listen_type_colum_sorter(PATH_PATH_COL, 0),
        SysdColumn::Path => create_socket_listen_type_colum_sorter(PATH_PATH_COL, 1),
        _ => create_custom_property_column_sorter(id),
    }
}

fn create_custom_property_column_sorter(id: &SysdColumn) -> Option<gtk::CustomSorter> {
    let key = id.generate_quark();

    let Some(prop_type) = id.property_type() else {
        warn!("column sorter without prop_type, id {:?}", key);
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

fn create_socket_listen_type_colum_sorter(qkey: &str, tuple_idx: u8) -> Option<gtk::CustomSorter> {
    let socket_listen = glib::Quark::from_str(qkey);
    let sorter = gtk::CustomSorter::new(move |o1, o2| {
        let v1 = o1
            .downcast_ref::<UnitInfo>()
            .map(|unit| extract_tuple_idx!(unit, socket_listen, tuple_idx));
        let v2 = o2
            .downcast_ref::<UnitInfo>()
            .map(|unit| extract_tuple_idx!(unit, socket_listen, tuple_idx));
        v1.into_iter().cmp(v2).into()
    });

    Some(sorter)
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
    let next_elapse_realtime_key = SysdColumn::TimerTimeNextElapseRT.generate_quark();
    let next_elapse_monotonic_key = SysdColumn::TimerTimeLeftElapseMono.generate_quark();

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

fn generate_default_columns(display_color: bool) -> Vec<UnitPropertySelection> {
    let mut columns = vec![];

    let unit_col = create_unit_display_name_column(display_color);
    columns.push(unit_col);

    let type_col = create_unit_type_column(display_color);
    columns.push(type_col);

    let sysd_col = SysdColumn::Bus;
    let sorter = create_column_filter!(dbus_level);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_bus(display_color);
    let bus_col = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(61)
        .title(pgettext("list column", "Bus"))
        .build();

    let bus_col = UnitPropertySelection::from_column_view_column(bus_col, sysd_col);
    columns.push(bus_col);

    let state_col = create_unit_file_state(display_color);
    columns.push(state_col);

    let preset_col = create_unit_file_preset_column(display_color);
    columns.push(preset_col);

    let sysd_col = SysdColumn::Load;
    let sorter = create_column_filter!(load_state);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_load_state(display_color);
    let load_col = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(80)
        .title(pgettext("list column", "Load"))
        .build();

    let load_col = UnitPropertySelection::from_column_view_column(load_col, sysd_col);
    columns.push(load_col);

    let active_col = create_unit_active_status_columun(display_color);
    columns.push(active_col);

    let sysd_col = SysdColumn::SubState;
    let sorter = create_column_filter!(sub_state);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_sub_state(display_color);
    let sub_col = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(71)
        .title(pgettext("list column", "Sub"))
        .build();

    let sub_col = UnitPropertySelection::from_column_view_column(sub_col, sysd_col);
    columns.push(sub_col);

    let sub_description = create_unit_description_column(display_color);
    columns.push(sub_description);

    columns
}

fn create_unit_file_preset_column(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::Preset;
    let sorter = create_column_filter!(preset);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_preset(display_color);

    let column = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(70)
        .title(pgettext("list column", "Preset"))
        .build();

    UnitPropertySelection::from_column_view_column(column, sysd_col)
}

fn create_unit_file_state(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::State;
    let sorter = create_column_filter!(enable_status);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_enable_status(display_color);

    let column = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(80)
        .title(pgettext("list column", "State"))
        .build();

    UnitPropertySelection::from_column_view_column(column, sysd_col)
}

fn create_unit_active_status_columun(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::Active;
    let sorter = create_column_filter!(active_state);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_active(display_color);

    let column = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(62)
        .title(pgettext("list column", "Active"))
        .build();

    UnitPropertySelection::from_column_view_column(column, sysd_col)
}

fn create_unit_description_column(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::Description;
    let sorter = create_column_filter!(description);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_descrition(display_color);

    let column = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .expand(true)
        .title(pgettext("list column", "Description"))
        .build();

    UnitPropertySelection::from_column_view_column(column, sysd_col)
}

fn create_unit_display_name_column(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::Name;
    let sorter = create_column_filter!(primary, dbus_level);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_unit_name(display_color);

    let column = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(150)
        //Unit full name column name
        .title(pgettext("list column", "Unit"))
        .build();

    UnitPropertySelection::from_column_view_column(column, sysd_col)
}

fn create_unit_display_full_name_column(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::FullName;
    let sorter = create_column_filter!(primary, dbus_level);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_unit_name(display_color);

    let col = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(150)
        .title(pgettext("list column", "Unit"))
        .build();

    UnitPropertySelection::from_column_view_column(col, sysd_col)
}

fn create_unit_type_column(display_color: bool) -> UnitPropertySelection {
    let sysd_col = SysdColumn::Type;
    let sorter = create_column_filter!(unit_type);
    let column_menu = create_col_menu(&sysd_col);
    let factory = fac_unit_type(display_color);

    let col = gtk::ColumnViewColumn::builder()
        .id(sysd_col.id())
        .sorter(&sorter)
        .header_menu(&column_menu)
        .factory(&factory)
        .resizable(true)
        .fixed_width(82)
        .title(pgettext("list column", "Type"))
        .build();

    UnitPropertySelection::from_column_view_column(col, sysd_col)
}

fn create_socket_listen_type_column() -> UnitColumn {
    let mut unit_column = UnitColumn::new(SOCKET_LISTEN_TYPE, "a(ss)");
    unit_column.resizable = true;
    //Socket list column name
    unit_column.title = Some(pgettext("list column", "Listen Type"));
    unit_column.fixed_width = 120;

    unit_column
}

fn create_socket_listen_column() -> UnitColumn {
    let mut unit_column = UnitColumn::new(SOCKET_LISTEN_COL, "a(ss)");
    unit_column.resizable = true;
    //Socket list column name
    unit_column.title = Some(pgettext("list column", "Listen"));
    unit_column.fixed_width = 80;
    unit_column.sort = Some(SortType::Asc);

    unit_column
}

fn create_time_next_time() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_NEXT, "t");
    unit_column.resizable = true;
    //Timer list column name
    unit_column.title = Some(pgettext("list column", "Next"));
    unit_column.fixed_width = 120;
    unit_column.sort = Some(SortType::Asc);
    unit_column
}

fn create_time_next_delay() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_LEFT, "t");
    unit_column.resizable = true;
    //Timer list column name
    unit_column.title = Some(pgettext("list column", "Left"));
    unit_column.fixed_width = 120;

    unit_column
}

fn create_time_last() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_LAST, "t");
    unit_column.resizable = true;
    //Timer list column name
    unit_column.title = Some(pgettext("list column", "Last"));
    unit_column.fixed_width = 120;
    unit_column
}

fn create_time_passed() -> UnitColumn {
    let mut unit_column = UnitColumn::new(TIMER_TIME_PASSED, "t");
    unit_column.resizable = true;
    //Timer list column name
    unit_column.title = Some(pgettext("list column", "Passed"));
    unit_column.fixed_width = 120;
    unit_column
}

fn create_col_activates() -> UnitColumn {
    let id = "unit@Triggers";

    let mut unit_column = UnitColumn::new(id, "as");
    unit_column.resizable = true;
    //Timer list column name
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
        .collect()
}
