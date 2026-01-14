use crate::{
    gtk::prelude::*,
    systemd::data::UnitInfo,
    widget::{
        unit_list::{
            imp::{
                column_factories::{self, *},
                construct,
            },
            menus::create_col_menu,
            CustomPropertyId, COL_ID_UNIT,
        },
        unit_properties_selector::{data_selection::UnitPropertySelection, save},
    },
};
use gettextrs::pgettext;
use log::warn;
use zvariant::Value;

pub fn construct_column(
    list_store: gio::ListStore,
    display_color: bool,
) -> (
    gtk::ColumnView,
    gtk::SingleSelection,
    gtk::FilterListModel,
    gtk::SortListModel,
    bool,
    Vec<UnitPropertySelection>,
) {
    let sort_list_model = gtk::SortListModel::new(Some(list_store), None::<gtk::Sorter>);
    let filter_list_model =
        gtk::FilterListModel::new(Some(sort_list_model.clone()), None::<gtk::Filter>);
    let selection_model = gtk::SingleSelection::builder()
        .model(&filter_list_model)
        .autoselect(false)
        .build();
    let column_view = gtk::ColumnView::new(Some(selection_model.clone()));

    let (base_columns, generated) = if let Some(saved_config) = save::load_column_config() {
        let mut list = Vec::with_capacity(saved_config.columns.len());
        for unit_column_config in saved_config.columns {
            let id = unit_column_config.id.clone();
            let prop_selection = UnitPropertySelection::from_column_config(unit_column_config);

            let column_menu = create_col_menu(&id, prop_selection.is_custom());
            let column = prop_selection.column();
            column.set_header_menu(Some(&column_menu));

            let prop_type = prop_selection.prop_type();

            construct::set_column_factory_and_sorter(&column, display_color, prop_type);

            list.push(prop_selection);
        }
        (list, false)
    } else {
        let default_columns = generate_default_columns(display_color);

        let mut column_view_column_definition_list = Vec::with_capacity(default_columns.len());

        for col in default_columns.iter() {
            let unit_property_selection: UnitPropertySelection =
                UnitPropertySelection::from_column_view_column(col.clone());
            column_view_column_definition_list.push(unit_property_selection);
        }

        (column_view_column_definition_list, true)
    };

    for col in base_columns.iter() {
        column_view.append_column(&col.column());
    }

    let sorter = column_view.sorter();
    sort_list_model.set_sorter(sorter.as_ref());

    (
        column_view,
        selection_model,
        filter_list_model,
        sort_list_model,
        generated,
        base_columns,
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

pub fn default_column_definition_list(display_color: bool) -> Vec<UnitPropertySelection> {
    generate_default_columns(display_color)
        .into_iter()
        .map(UnitPropertySelection::from_column_view_column)
        .collect()
}

pub fn set_column_factory_and_sorter(
    column: &gtk::ColumnViewColumn,
    display_color: bool,
    prop_type: Option<String>,
) {
    let Some(id) = column.id() else {
        warn!("No column id");
        return;
    };

    //identify custom properties
    let custom_id = CustomPropertyId::from_str(id.as_str());

    //force data display
    let factory = column_factories::get_factory_by_id(&custom_id, display_color, &prop_type);
    column.set_factory(factory.as_ref());

    let sorter = get_sorter_by_id(custom_id, &prop_type);
    column.set_sorter(sorter.as_ref());
}

pub fn get_sorter_by_id(
    id: CustomPropertyId,
    prop_type: &Option<String>,
) -> Option<gtk::CustomSorter> {
    match id.prop {
        COL_ID_UNIT => Some(create_column_filter!(primary, dbus_level)),
        "sysdm-type" => Some(create_column_filter!(unit_type)),
        "sysdm-bus" => Some(create_column_filter!(dbus_level)),
        "sysdm-state" => Some(create_column_filter!(enable_status)),
        "sysdm-preset" => Some(create_column_filter!(preset)),
        "sysdm-load" => Some(create_column_filter!(load_state)),
        "sysdm-active" => Some(create_column_filter!(active_state)),
        "sysdm-sub" => Some(create_column_filter!(sub_state)),
        "sysdm-description" => Some(create_column_filter!(description)),

        _ => create_custom_property_column_sorter(id, prop_type),
    }
}

fn create_custom_property_column_sorter(
    id: CustomPropertyId,
    prop_type: &Option<String>,
) -> Option<gtk::CustomSorter> {
    let key = id.generate_quark();

    let Some(prop_type) = prop_type else {
        warn!("column sorter without prop_type ");
        return None;
    };

    let sort_func = match prop_type.as_str() {
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
        .title(pgettext("list column", "Unit"))
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
        .title(pgettext("list column", "Type"))
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
        .title(pgettext("list column", "Bus"))
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
        .title(pgettext("list column", "State"))
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
        .title(pgettext("list column", "Preset"))
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
        .title(pgettext("list column", "Load"))
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
        .title(pgettext("list column", "Active"))
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
        .title(pgettext("list column", "Sub"))
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
        .title(pgettext("list column", "Description"))
        .build();
    columns.push(sub_description);

    columns
}
