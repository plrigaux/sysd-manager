use std::{sync::LazyLock, u64};

use gtk::{
    glib::{self, Binding, Quark},
    prelude::*,
};
use log::{error, warn};
use systemd::time_handling::{self, calc_next_elapse};
use zvariant::OwnedValue;

use crate::{
    consts::{
        NEXT_ELAPSE_USEC_MONOTONIC, NEXT_ELAPSE_USEC_REALTIME, TIMER_TIME_LAST, TIMER_TIME_LEFT,
        TIMER_TIME_NEXT, TIMER_TIME_PASSED, U64MAX,
    },
    widget::{
        preferences::data::PREFERENCES,
        unit_list::{COL_ID_UNIT, CustomPropertyId},
        unit_properties_selector::data_selection::UnitPropertySelection,
    },
};
use crate::{
    systemd::{
        data::{UnitInfo, convert_to_string},
        enums::{ActiveState, LoadState, Preset, UnitFileStatus},
    },
    widget::unit_list::UnitListPanel,
};

static BIND_INFO: LazyLock<Quark> = LazyLock::new(|| Quark::from_str("I"));
static BIND_CSS: LazyLock<Quark> = LazyLock::new(|| Quark::from_str("C"));
static BIND_CSS2: LazyLock<Quark> = LazyLock::new(|| Quark::from_str("C2"));

const CSS_CLASSES: &str = "css-classes";
const TEXT: &str = "text";

macro_rules! downcast_list_item {
    ($list_item_object:expr) => {{
        $list_item_object
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()")
    }};
}

fn factory_setup(_factory: &gtk::SignalListItemFactory, object: &glib::Object) {
    let list_item = downcast_list_item!(object);
    let inscription = gtk::Inscription::builder()
        //        .xalign(0.0)
        .wrap_mode(gtk::pango::WrapMode::None)
        .build();

    list_item.set_child(Some(&inscription));
}

macro_rules! factory_bind_pre {
    ($list_item_object:expr) => {{
        let list_item = downcast_list_item!($list_item_object);
        let inscription = list_item
            .child()
            .and_downcast::<gtk::Inscription>()
            .expect("item.downcast_ref::<gtk::Inscription>()");
        let unit_binding = list_item
            .item()
            .and_downcast::<UnitInfo>()
            .expect("item.downcast_ref::<gtk::UnitBinding>()");
        (inscription, unit_binding)
    }};
}

macro_rules! factory_bind {
    ($item_obj:expr, $func:ident) => {{
        let (inscription, unit) = factory_bind_pre!($item_obj);
        let text = $func(&unit);
        inscription.set_text(Some(&text));
        (inscription, unit)
    }};
}

macro_rules! factory_bind_enum {
    ($item_obj:expr, $func:ident) => {{
        let (inscription, unit) = factory_bind_pre!($item_obj);
        let text = unit.$func().as_str();
        inscription.set_text(Some(text));
        (inscription, unit)
    }};
}

const ACTIVE_STATE: &str = "active_state";
const CSS_GREY: &str = "grey";

macro_rules! display_inactive {
    ($widget:expr, $unit:expr) => {
        let state = $unit.active_state();
        if state.is_inactive() {
            $widget.set_css_classes(&[CSS_GREY]);
        } else {
            $widget.set_css_classes(&[]);
        }
    };
}

macro_rules! factory_connect_unbind {
    ($factory:expr, $($bind_id:expr), *) => {
        $factory.connect_unbind(|_factory, object| {
            let list_item = downcast_list_item!(object);
            let Some(child) = list_item.child() else {
                warn!("No child");
                return;
            };
            $(
                unbind(&child, $bind_id);
            )*
        });
    };
}

fn store_binding(object: &impl IsA<gtk::Widget>, key: Quark, binding: Binding) {
    unsafe {
        object.set_qdata(key, binding);
    }
}

fn unbind(child: &gtk::Widget, key: Quark) {
    let binding: Option<Binding> = unsafe { child.steal_qdata(key) };
    if let Some(binding) = binding {
        binding.unbind();
    }
}

fn inactive_display(widget: &impl IsA<gtk::Widget>, unit: &UnitInfo) {
    display_inactive!(widget, unit);

    let binding = unit
        .bind_property(ACTIVE_STATE, widget, CSS_CLASSES)
        .transform_to(|_, active_state: ActiveState| {
            let css_classes = if active_state.is_inactive() {
                [CSS_GREY].to_value()
            } else {
                [].to_value()
            };
            Some(css_classes)
        })
        .build();

    store_binding(widget, *BIND_CSS, binding);
}

pub fn fac_unit_name(display_color: bool) -> gtk::SignalListItemFactory {
    common_factory(display_color, UnitInfo::display_name)
}

fn common_factory(
    display_color: bool,
    func: fn(&UnitInfo) -> String,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(factory_setup);

    if display_color {
        factory.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind!(object, func);
            inactive_display(&inscription, &unit)
        });

        factory_connect_unbind!(factory, *BIND_CSS);
    } else {
        factory.connect_bind(move |_factory, object| {
            factory_bind!(object, func);
        });
    }
    factory
}

pub fn fac_unit_type(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_unit_type = gtk::SignalListItemFactory::new();

    fac_unit_type.connect_setup(factory_setup);
    if display_color {
        fac_unit_type.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, unit_type);
            inactive_display(&inscription, &unit)
        });
        factory_connect_unbind!(fac_unit_type, *BIND_CSS);
    } else {
        fac_unit_type.connect_bind(move |_factory, object| {
            factory_bind_enum!(object, unit_type);
        });
    }
    fac_unit_type
}

pub fn fac_bus(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_bus = gtk::SignalListItemFactory::new();

    fac_bus.connect_setup(factory_setup);
    if display_color {
        fac_bus.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, dbus_level);
            inactive_display(&inscription, &unit)
        });
        factory_connect_unbind!(fac_bus, *BIND_CSS);
    } else {
        fac_bus.connect_bind(move |_factory, object| {
            factory_bind_enum!(object, dbus_level);
        });
    }
    fac_bus
}

pub fn fac_active(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_active = gtk::SignalListItemFactory::new();

    fac_active.connect_setup(|_factory, object| {
        let item = downcast_list_item!(object);
        let image = gtk::Image::new();
        item.set_child(Some(&image));
    });

    if display_color {
        fac_active.connect_bind(|_factory, object| {
            let (icon_image, unit) = active_icon(object);
            inactive_display(&icon_image, &unit)
        });

        factory_connect_unbind!(&fac_active, *BIND_INFO, *BIND_CSS);
    } else {
        fac_active.connect_bind(|_factory, object| {
            active_icon(object);
        });
        factory_connect_unbind!(&fac_active, *BIND_INFO);
    }
    fac_active
}

fn active_icon(object: &glib::Object) -> (gtk::Image, UnitInfo) {
    let list_item: &gtk::ListItem = downcast_list_item!(object);
    let icon_image = list_item.child().and_downcast::<gtk::Image>().unwrap();
    let unit = list_item.item().and_downcast::<UnitInfo>().unwrap();
    let state = unit.active_state();

    let icon_name = state.icon_name();
    icon_image.set_icon_name(icon_name);
    icon_image.set_tooltip_text(Some(state.as_str()));

    let binding = unit
        .bind_property("active_state", &icon_image, "icon-name")
        .transform_to(|_, state: ActiveState| state.icon_name())
        .build();

    store_binding(&icon_image, *BIND_INFO, binding);
    (icon_image, unit)
}

pub fn fac_sub_state(display_color: bool) -> gtk::SignalListItemFactory {
    common_factory(display_color, UnitInfo::sub_state)
}

pub fn fac_descrition(display_color: bool) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(factory_setup);

    if display_color {
        factory.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_pre!(object);
            let text = UnitInfo::description(&unit);
            inscription.set_text(text.as_deref());

            inactive_display(&inscription, &unit)
        });

        factory_connect_unbind!(factory, *BIND_CSS);
    } else {
        factory.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_pre!(object);
            let text = UnitInfo::description(&unit);
            inscription.set_text(text.as_deref());
        });
    }
    factory
}

pub fn setup_factories(
    unit_list: &UnitListPanel,
    column_view_column_list: &Vec<gtk::ColumnViewColumn>,
    current_column_view_column_definition_list: &[UnitPropertySelection],
) {
    let display_color = unit_list.display_color();

    for column in column_view_column_list {
        let Some(id) = column.id() else {
            warn!("Column with no id!");
            continue;
        };

        let id = id.as_str();
        let prop_type =
            current_column_view_column_definition_list
                .iter()
                .find_map(|prop_selection| {
                    if prop_selection.id().is_some_and(|s| id == s.as_str()) {
                        prop_selection.prop_type()
                    } else {
                        None
                    }
                });

        let custom_id = CustomPropertyId::from_str(id);
        let factory = get_factory_by_id(&custom_id, display_color, prop_type.as_deref());

        column.set_factory(factory.as_ref());
    }
}

pub fn get_factory_by_id(
    id: &CustomPropertyId,
    display_color: bool,
    prop_type: Option<&str>,
) -> Option<gtk::SignalListItemFactory> {
    match (id.has_defined_type(), id.prop) {
        (true, _) => Some(get_custom_factory(id, display_color, prop_type)),
        (false, COL_ID_UNIT) => Some(fac_unit_name(display_color)),
        (false, "sysdm-type") => Some(fac_unit_type(display_color)),
        (false, "sysdm-bus") => Some(fac_bus(display_color)),
        (false, "sysdm-state") => Some(fac_enable_status(display_color)),
        (false, "sysdm-preset") => Some(fac_preset(display_color)),
        (false, "sysdm-load") => Some(fac_load_state(display_color)),
        (false, "sysdm-active") => Some(fac_active(display_color)),
        (false, "sysdm-sub") => Some(fac_sub_state(display_color)),
        (false, "sysdm-description") => Some(fac_descrition(display_color)),
        (false, TIMER_TIME_NEXT) => Some(fac_time_next()),
        (false, TIMER_TIME_LEFT) => Some(fac_time_left()),
        (false, TIMER_TIME_PASSED) => Some(fac_time_passed()),
        (false, TIMER_TIME_LAST) => Some(fac_time_last()),
        _ => {
            warn!("What to do?. Id {id:?} not handle with factory");
            None
        }
    }
}

const LOAD_STATE: &str = "load_state";
pub fn fac_load_state(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_load_state = gtk::SignalListItemFactory::new();

    fac_load_state.connect_setup(factory_setup);

    if display_color {
        fac_load_state.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, load_state);

            load_state_text_binding(&inscription, &unit);

            let binding = unit
                .bind_property(LOAD_STATE, &inscription, CSS_CLASSES)
                .transform_to(|_, load_state: LoadState| {
                    let css_classes = load_state_css_classes(load_state);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            store_binding(&inscription, *BIND_CSS, binding);

            let binding = unit
                .bind_property(ACTIVE_STATE, &inscription, CSS_CLASSES)
                .transform_to(|_, active_state: ActiveState| {
                    let css_classes = if active_state.is_inactive() {
                        [CSS_GREY].to_value()
                    } else {
                        [].to_value()
                    };
                    Some(css_classes)
                })
                .build();

            store_binding(&inscription, *BIND_CSS2, binding);

            let load_state = unit.load_state();
            inscription.set_text(Some(load_state.as_str()));

            let css_classes = load_state_css_classes(load_state);
            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
            }
        });
        factory_connect_unbind!(&fac_load_state, *BIND_INFO, *BIND_CSS, *BIND_CSS2);
    } else {
        fac_load_state.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, load_state);
            load_state_text_binding(&inscription, &unit);
        });

        factory_connect_unbind!(&fac_load_state, *BIND_INFO);
    }

    fac_load_state
}

fn load_state_text_binding(inscription: &gtk::Inscription, unit: &UnitInfo) {
    let binding = unit
        .bind_property(LOAD_STATE, inscription, TEXT)
        .transform_to(|_, load_state: LoadState| Some(load_state.as_str()))
        .build();
    store_binding(inscription, *BIND_INFO, binding);
}

fn load_state_css_classes<'a>(load_state: LoadState) -> Option<[&'a str; 2]> {
    match load_state {
        LoadState::NotFound => Some(["yellow", "bold"]),
        LoadState::BadSetting | LoadState::Error | LoadState::Masked => Some(["red", "bold"]),
        _ => None,
    }
}

const ENABLE_STATUS: &str = "enable_status";
pub fn fac_enable_status(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_enable_status = gtk::SignalListItemFactory::new();

    fac_enable_status.connect_setup(factory_setup);

    if display_color {
        fac_enable_status.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_pre!(object);

            let status_code = unit.enable_status();
            inscription.set_text(Some(status_code.as_str()));
            inscription.set_tooltip_markup(status_code.tooltip_info().as_deref());

            let binding = unit
                .bind_property(ENABLE_STATUS, &inscription, TEXT)
                .transform_to(|_, enablement_status: UnitFileStatus| {
                    Some(enablement_status.as_str())
                })
                .build();

            store_binding(&inscription, *BIND_INFO, binding);

            let binding = unit
                .bind_property(ENABLE_STATUS, &inscription, CSS_CLASSES)
                .transform_to(|_, enablement_status: UnitFileStatus| {
                    let css_classes = enablement_css_classes(enablement_status);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            store_binding(&inscription, *BIND_CSS, binding);

            let css_classes = enablement_css_classes(status_code);

            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
            }
        });

        factory_connect_unbind!(&fac_enable_status, *BIND_INFO, *BIND_CSS);
    } else {
        fac_enable_status.connect_bind(move |_factory, object| {
            factory_bind_enum!(object, enable_status);
        });
    }
    fac_enable_status
}

fn enablement_css_classes<'a>(enablement_status: UnitFileStatus) -> Option<[&'a str; 2]> {
    match enablement_status {
        UnitFileStatus::Bad
        | UnitFileStatus::Disabled
        | UnitFileStatus::Masked
        | UnitFileStatus::MaskedRuntime => Some(["red", "bold"]),

        UnitFileStatus::Alias | UnitFileStatus::Enabled | UnitFileStatus::EnabledRuntime => {
            Some(["green", "bold"])
        }

        _ => None,
    }
}

const PRESET_NUM: &str = "preset";

pub fn fac_preset(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_preset = gtk::SignalListItemFactory::new();

    fac_preset.connect_setup(factory_setup);

    if display_color {
        fac_preset.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, preset);
            preset_text_binding(&inscription, &unit);

            let binding = unit
                .bind_property(PRESET_NUM, &inscription, CSS_CLASSES)
                .transform_to(|_s, preset_value: Preset| {
                    let css_classes = preset_css_classes(preset_value);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            store_binding(&inscription, *BIND_CSS, binding);

            let preset_value = unit.preset();
            let css_classes = preset_css_classes(preset_value);

            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
                inscription.set_css_classes(&[]);
            }
        });

        factory_connect_unbind!(&fac_preset, *BIND_INFO, *BIND_CSS);
    } else {
        fac_preset.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_enum!(object, preset);
            preset_text_binding(&inscription, &unit);
        });
        factory_connect_unbind!(&fac_preset, *BIND_INFO);
    }
    fac_preset
}

fn preset_text_binding(inscription: &gtk::Inscription, unit: &UnitInfo) {
    let binding = unit
        .bind_property(PRESET_NUM, inscription, TEXT)
        .transform_to(|_s, preset: Preset| preset.as_str_op())
        .build();
    store_binding(inscription, *BIND_INFO, binding);
}

fn preset_css_classes(preset_value: Preset) -> Option<[&'static str; 2]> {
    match preset_value {
        Preset::Disabled => Some(["red", "bold"]),
        Preset::Enabled => Some(["green", "bold"]),
        Preset::Ignore => Some(["yellow", "bold"]),
        _ => None,
    }
}

pub(super) fn get_custom_factory(
    property_code: &CustomPropertyId,
    display_color: bool,
    prop_type: Option<&str>,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let key = property_code.quark();
    factory.connect_setup(factory_setup);

    let Some(prop_type) = prop_type else {
        error!("NO PROP_TYPE SET for {:?}", property_code);
        return factory;
    };

    let get_value = match prop_type {
        "b" => get_custom_property_typed::<bool>,
        "n" => get_custom_property_typed::<i16>,
        "q" => get_custom_property_typed::<u16>,
        "i" => get_custom_property_typed::<i32>,
        "u" => get_custom_property_typed::<u32>,
        "s" => get_custom_property_typed::<String>,
        "x" => get_custom_property_typed::<i64>,
        "t" => get_custom_property_typed::<u64>,
        "v" => display_custom_property,
        _ => get_custom_property_typed::<String>,
    };

    if display_color {
        factory.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_pre!(object);
            inactive_display(&inscription, &unit);
            let value = get_value(key, &unit);
            inscription.set_text(value.as_deref());
        });
    } else {
        factory.connect_bind(move |_factory, object| {
            let (inscription, unit) = factory_bind_pre!(object);
            let value = get_value(key, &unit);
            inscription.set_text(value.as_deref());
        });
    }

    factory
}

fn get_custom_property_typed<T>(key: Quark, unit: &UnitInfo) -> Option<String>
where
    T: ToString + 'static,
{
    unsafe { unit.qdata::<T>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .map(|value| value.to_string())
}

fn get_custom_property_typed_raw<T>(key: Quark, unit: &UnitInfo) -> Option<T>
where
    T: Copy + 'static,
{
    unsafe { unit.qdata::<T>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .copied()
}

fn display_custom_property(key: Quark, unit: &UnitInfo) -> Option<String> {
    unsafe { unit.qdata::<OwnedValue>(key) }
        .map(|value_ptr| unsafe { value_ptr.as_ref() })
        .and_then(|value| convert_to_string(value))
}

fn fac_time_last() -> gtk::SignalListItemFactory {
    let time_fac = gtk::SignalListItemFactory::new();

    time_fac.connect_setup(factory_setup);
    time_fac
}

// ///from systemd code
// fn calc_next_elapse(next_elapse_realtime: Option<u64>, next_elapse_monotonic: Option<u64>) -> u64 {
//     let now_realtime = time_handling::now_realtime();
//     let now_monotonic = time_handling::now_monotonic();
//     if let Some(next_elapse_monotonic) = next_elapse_monotonic
//         && next_elapse_monotonic != u64::MAX
//     {
//         let converted = if next_elapse_monotonic > now_monotonic {
//             now_realtime + (next_elapse_monotonic - now_monotonic)
//         } else {
//             now_realtime - (now_monotonic - next_elapse_monotonic)
//         };

//         if let Some(next_elapse_realtime) = next_elapse_realtime
//             && next_elapse_realtime != u64::MAX
//         {
//             converted.min(next_elapse_realtime)
//         } else {
//             converted
//         }
//     } else {
//         next_elapse_realtime.unwrap_or(u64::MAX)
//     }
// }

fn fac_time_passed() -> gtk::SignalListItemFactory {
    let time_fac = gtk::SignalListItemFactory::new();

    time_fac.connect_setup(factory_setup);
    let key = Quark::from_str(NEXT_ELAPSE_USEC_REALTIME);
    time_fac.connect_bind(move |_factory, object| {
        let (inscription, unit) = factory_bind_pre!(object);
        inactive_display(&inscription, &unit);
        let value = get_custom_property_typed::<u64>(key, &unit);
        inscription.set_text(value.as_deref());
    });
    time_fac
}

fn fac_time_next() -> gtk::SignalListItemFactory {
    let time_fac = gtk::SignalListItemFactory::new();

    time_fac.connect_setup(factory_setup);
    let next_elapse_realtime_key = Quark::from_str(NEXT_ELAPSE_USEC_REALTIME);
    let next_elapse_monotonic_key = Quark::from_str(NEXT_ELAPSE_USEC_MONOTONIC);
    let timestamp_style = PREFERENCES.timestamp_style();
    time_fac.connect_bind(move |_factory, object| {
        let (inscription, unit) = factory_bind_pre!(object);
        inactive_display(&inscription, &unit);

        let next_elapse_realtime =
            get_custom_property_typed_raw::<u64>(next_elapse_realtime_key, &unit).unwrap_or(U64MAX);
        let next_elapse_monotonic =
            get_custom_property_typed_raw::<u64>(next_elapse_monotonic_key, &unit)
                .unwrap_or(U64MAX);

        let next_elapse = calc_next_elapse(next_elapse_realtime, next_elapse_monotonic);
        if next_elapse != u64::MAX {
            let x = time_handling::get_since_and_passed_time(next_elapse, timestamp_style);
            inscription.set_text(Some(&x.0));
        } else {
            inscription.set_text(None);
        }
    });
    time_fac
}

fn fac_time_left() -> gtk::SignalListItemFactory {
    let time_fac = gtk::SignalListItemFactory::new();

    time_fac.connect_setup(factory_setup);
    let next_elapse_realtime_key = Quark::from_str(NEXT_ELAPSE_USEC_REALTIME);
    let next_elapse_monotonic_key = Quark::from_str(NEXT_ELAPSE_USEC_MONOTONIC);
    let timestamp_style = PREFERENCES.timestamp_style();
    time_fac.connect_bind(move |_factory, object| {
        let (inscription, unit) = factory_bind_pre!(object);
        inactive_display(&inscription, &unit);

        let next_elapse_realtime =
            get_custom_property_typed_raw::<u64>(next_elapse_realtime_key, &unit).unwrap_or(U64MAX);
        let next_elapse_monotonic =
            get_custom_property_typed_raw::<u64>(next_elapse_monotonic_key, &unit)
                .unwrap_or(U64MAX);

        let next_elapse = calc_next_elapse(next_elapse_realtime, next_elapse_monotonic);
        if next_elapse != u64::MAX {
            let x = time_handling::get_since_and_passed_time(next_elapse, timestamp_style);
            inscription.set_text(Some(&x.1));
        } else {
            inscription.set_text(None);
        }
    });

    time_fac
}
