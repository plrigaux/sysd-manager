use std::collections::HashMap;

use gtk::{glib, prelude::*};
use log::debug;

use crate::systemd::UnitProperty;
use crate::systemd::data::UnitInfo;
use crate::systemd::enums::{EnablementStatus, LoadState, Preset};
use crate::widget::unit_list::imp::rowdata::UnitBinding;
use crate::{systemd::enums::ActiveState, widget::unit_list::UnitListPanel};

pub const BIND_DESCRIPTION_TEXT: u8 = 0;
pub const BIND_SUB_STATE_TEXT: u8 = 1;
pub const BIND_ENABLE_STATUS_TEXT: u8 = 2;
pub const BIND_ENABLE_STATUS_CSS: u8 = 3;
pub const BIND_ENABLE_PRESET_TEXT: u8 = 4;
pub const BIND_ENABLE_PRESET_CSS: u8 = 5;
pub const BIND_ENABLE_LOAD_TEXT: u8 = 6;
pub const BIND_ENABLE_LOAD_CSS: u8 = 7;
pub const BIND_ENABLE_ACTIVE_ICON: u8 = 8;

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

macro_rules! downcast_unit_binding {
    ($list_item_object:expr) => {{
        let list_item = downcast_list_item!($list_item_object);
        list_item
            .item()
            .and_downcast::<UnitBinding>()
            .expect("item.downcast_ref::<gtk::UnitBinding>()")
    }};
}

macro_rules! factory_connect_unbind {
    ($factory:expr, $($bind_id:expr), *) => {
        $factory.connect_unbind(|_factory, object| {
            let unit_binding = downcast_unit_binding!(object);
            $(
                unit_binding.unset_binding($bind_id);
            )*
        });
    };
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
            .and_downcast::<UnitBinding>()
            .expect("item.downcast_ref::<gtk::UnitBinding>()");
        (inscription, unit_binding)
    }};
}

macro_rules! factory_bind {
    ($item_obj:expr, $func:ident) => {{
        let (inscription, unit_binding) = factory_bind_pre!($item_obj);
        let unit = unit_binding.unit();
        let text = unit.$func();
        inscription.set_text(Some(&text));
        (inscription, unit, unit_binding)
    }};
}

macro_rules! factory_bind_enum {
    ($item_obj:expr, $func:ident) => {{
        let (inscription, unit_binding) = factory_bind_pre!($item_obj);
        let unit = unit_binding.unit();
        let text = unit.$func().as_str();
        inscription.set_text(Some(&text));
        (inscription, unit, unit_binding)
    }};
}

//TODO bind properties
macro_rules! display_inactive {
    ($widget:expr, $unit:expr) => {
        let state = &$unit.active_state();
        if state.is_inactive() {
            $widget.set_css_classes(&["grey"]);
        } else {
            $widget.set_css_classes(&[]);
        }
    };
}

pub fn setup_factories(
    unit_list: &UnitListPanel,
    column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>,
) {
    let display_color = unit_list.display_color();
    let fac_unit_name = gtk::SignalListItemFactory::new();

    fac_unit_name.connect_setup(factory_setup);

    {
        //let unit_list = unit_list.clone();
        fac_unit_name.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind!(object, display_name);
            display_inactive!(inscription, unit);
        });
    }

    let fac_unit_type = gtk::SignalListItemFactory::new();

    fac_unit_type.connect_setup(factory_setup);

    {
        //let unit_list = unit_list.clone();
        fac_unit_type.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind_enum!(object, unit_type);
            display_inactive!(inscription, unit);
        });
    }

    let fac_bus = gtk::SignalListItemFactory::new();

    fac_bus.connect_setup(factory_setup);

    {
        fac_bus.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind_enum!(object, dbus_level);
            display_inactive!(inscription, unit);
        });
    }

    let fac_enable_status = fac_enable_status(display_color);
    let fac_preset = fac_preset(display_color);
    let fac_load_state = fac_load_state(display_color);

    let fac_active = gtk::SignalListItemFactory::new();

    fac_active.connect_setup(|_factory, object| {
        let item = downcast_list_item!(object);
        let image = gtk::Image::new();
        item.set_child(Some(&image));
    });

    fac_active.connect_bind(|_factory, object| {
        let list_item: &gtk::ListItem = downcast_list_item!(object);
        let icon_image = list_item.child().and_downcast::<gtk::Image>().unwrap();

        let unit_binding = list_item.item().and_downcast::<UnitBinding>().unwrap();
        let unit = unit_binding.unit_ref();

        let state = unit.active_state();

        let icon_name = state.icon_name();
        icon_image.set_icon_name(icon_name);
        icon_image.set_tooltip_text(Some(state.as_str()));

        let binding = unit
            .bind_property("active_state", &icon_image, "icon-name")
            .transform_to(|_, state: ActiveState| state.icon_name())
            .build();

        unit_binding.set_binding(BIND_ENABLE_ACTIVE_ICON, binding);
        display_inactive!(icon_image, unit);
    });

    factory_connect_unbind!(fac_active, BIND_ENABLE_ACTIVE_ICON);

    let fac_sub_state = gtk::SignalListItemFactory::new();

    fac_sub_state.connect_setup(factory_setup);

    fac_sub_state.connect_bind(|_factory, object| {
        let (inscription, unit, unit_binding) = factory_bind!(object, sub_state);
        let binding = unit.bind_property("sub_state", &inscription, TEXT).build();
        unit_binding.set_binding(BIND_SUB_STATE_TEXT, binding);
        display_inactive!(inscription, unit);
    });

    factory_connect_unbind!(fac_sub_state, BIND_SUB_STATE_TEXT);

    let fac_descrition = gtk::SignalListItemFactory::new();

    fac_descrition.connect_setup(factory_setup);

    fac_descrition.connect_bind(|_factory, object| {
        let (inscription, unit, unit_binding) = factory_bind!(object, description);
        let binding = unit
            .bind_property("description", &inscription, TEXT)
            .build();
        unit_binding.set_binding(BIND_DESCRIPTION_TEXT, binding);
        display_inactive!(inscription, unit);
    });

    fac_descrition.connect_unbind(|_factory, object| {
        let unit_binding = downcast_unit_binding!(object);
        unit_binding.unset_binding(BIND_DESCRIPTION_TEXT);
    });

    factory_connect_unbind!(fac_descrition, BIND_DESCRIPTION_TEXT);

    column_view_column_map
        .get("unit")
        .unwrap()
        .set_factory(Some(&fac_unit_name));
    column_view_column_map
        .get("type")
        .unwrap()
        .set_factory(Some(&fac_unit_type));
    column_view_column_map
        .get("bus")
        .unwrap()
        .set_factory(Some(&fac_bus));
    column_view_column_map
        .get("state")
        .unwrap()
        .set_factory(Some(&fac_enable_status));
    column_view_column_map
        .get("preset")
        .unwrap()
        .set_factory(Some(&fac_preset));
    column_view_column_map
        .get("load")
        .unwrap()
        .set_factory(Some(&fac_load_state));
    column_view_column_map
        .get("active")
        .unwrap()
        .set_factory(Some(&fac_active));
    column_view_column_map
        .get("sub")
        .unwrap()
        .set_factory(Some(&fac_sub_state));
    column_view_column_map
        .get("description")
        .unwrap()
        .set_factory(Some(&fac_descrition));

    for cv_column in column_view_column_map.values() {
        cv_column.connect_fixed_width_notify(|cvc| {
            debug!(
                "Column width {:?} {}",
                cvc.id().unwrap_or_default(),
                cvc.fixed_width()
            )
        });
    }
}

const LOAD_STATE: &str = "load_state";
fn fac_load_state(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_load_state = gtk::SignalListItemFactory::new();

    fac_load_state.connect_setup(factory_setup);

    if display_color {
        fac_load_state.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind_enum!(object, load_state);

            load_state_text_binding(&inscription, &unit_binding, &unit);

            let binding = unit
                .bind_property(LOAD_STATE, &inscription, CSS_CLASSES)
                .transform_to(|_, load_state: LoadState| {
                    let css_classes = load_state_css_classes(load_state);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_LOAD_CSS, binding);

            let load_state = unit.load_state();
            inscription.set_text(Some(load_state.as_str()));

            let css_classes = load_state_css_classes(load_state);
            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
            }
        });
        factory_connect_unbind!(fac_load_state, BIND_ENABLE_LOAD_TEXT, BIND_ENABLE_LOAD_CSS);
    } else {
        fac_load_state.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind_enum!(object, load_state);
            load_state_text_binding(&inscription, &unit_binding, &unit);
            display_inactive!(inscription, unit);
        });

        factory_connect_unbind!(fac_load_state, BIND_ENABLE_LOAD_TEXT);
    }

    fac_load_state
}

fn load_state_text_binding(
    inscription: &gtk::Inscription,
    unit_binding: &UnitBinding,
    unit: &UnitInfo,
) {
    let binding = unit
        .bind_property(LOAD_STATE, inscription, TEXT)
        .transform_to(|_, load_state: LoadState| Some(load_state.as_str()))
        .build();
    unit_binding.set_binding(BIND_ENABLE_LOAD_TEXT, binding);
}

fn load_state_css_classes<'a>(load_state: LoadState) -> Option<[&'a str; 2]> {
    match load_state {
        LoadState::NotFound => Some(["yellow", "bold"]),
        LoadState::BadSetting | LoadState::Error | LoadState::Masked => Some(["red", "bold"]),
        _ => None,
    }
}

const ENABLE_STATUS: &str = "enable_status";
fn fac_enable_status(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_enable_status = gtk::SignalListItemFactory::new();

    fac_enable_status.connect_setup(factory_setup);

    if display_color {
        fac_enable_status.connect_bind(move |_factory, object| {
            let (inscription, unit_binding) = factory_bind_pre!(object);

            let unit = unit_binding.unit_ref();
            let status_code = unit.enable_status();
            inscription.set_text(Some(status_code.as_str()));
            inscription.set_tooltip_markup(status_code.tooltip_info().as_deref());

            let binding = unit
                .bind_property(ENABLE_STATUS, &inscription, TEXT)
                .transform_to(|_, enablement_status: EnablementStatus| {
                    Some(enablement_status.as_str())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_STATUS_TEXT, binding);

            let binding = unit
                .bind_property(ENABLE_STATUS, &inscription, CSS_CLASSES)
                .transform_to(|_, enablement_status: EnablementStatus| {
                    let css_classes = enablement_css_classes(enablement_status);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_STATUS_CSS, binding);

            let css_classes = enablement_css_classes(status_code);

            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
            }
        });

        factory_connect_unbind!(
            fac_enable_status,
            BIND_ENABLE_STATUS_TEXT,
            BIND_ENABLE_STATUS_CSS
        );
    } else {
        fac_enable_status.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind_enum!(object, enable_status);

            let binding = unit
                .bind_property(ENABLE_STATUS, &inscription, TEXT)
                .transform_to(|_, status: EnablementStatus| Some(status.as_str()))
                .build();

            unit_binding.set_binding(BIND_ENABLE_STATUS_TEXT, binding);
            display_inactive!(inscription, unit);
        });

        factory_connect_unbind!(fac_enable_status, BIND_ENABLE_STATUS_TEXT);
    }
    fac_enable_status
}

fn enablement_css_classes<'a>(enablement_status: EnablementStatus) -> Option<[&'a str; 2]> {
    match enablement_status {
        EnablementStatus::Bad
        | EnablementStatus::Disabled
        | EnablementStatus::Masked
        | EnablementStatus::MaskedRuntime => Some(["red", "bold"]),

        EnablementStatus::Alias | EnablementStatus::Enabled | EnablementStatus::EnabledRuntime => {
            Some(["green", "bold"])
        }

        _ => None,
    }
}

const PRESET_NUM: &str = "preset";

fn fac_preset(display_color: bool) -> gtk::SignalListItemFactory {
    let fac_preset = gtk::SignalListItemFactory::new();

    fac_preset.connect_setup(factory_setup);

    if display_color {
        fac_preset.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind_enum!(object, preset);

            preset_text_binding(&inscription, &unit, &unit_binding);

            let binding = unit
                .bind_property(PRESET_NUM, &inscription, CSS_CLASSES)
                .transform_to(move |_s, preset_value: Preset| {
                    let css_classes = preset_css_classes(preset_value);
                    css_classes.map(|css| css.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_PRESET_CSS, binding);

            let preset_value = unit.preset();
            let css_classes = preset_css_classes(preset_value);

            if let Some(css) = css_classes {
                inscription.set_css_classes(&css);
            } else {
                display_inactive!(inscription, unit);
            }
        });

        factory_connect_unbind!(fac_preset, BIND_ENABLE_PRESET_TEXT, BIND_ENABLE_PRESET_CSS);
    } else {
        fac_preset.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind_enum!(object, preset);

            preset_text_binding(&inscription, &unit, &unit_binding);
            display_inactive!(inscription, unit);
        });

        factory_connect_unbind!(fac_preset, BIND_ENABLE_PRESET_TEXT);
    }
    fac_preset
}

fn preset_text_binding(
    inscription: &gtk::Inscription,
    unit: &UnitInfo,
    unit_binding: &UnitBinding,
) {
    let binding = unit
        .bind_property(PRESET_NUM, inscription, TEXT)
        .transform_to(move |_s, preset: Preset| {
            let preset_str = preset.as_str();
            Some(preset_str.to_value())
        })
        .build();
    unit_binding.set_binding(BIND_ENABLE_PRESET_TEXT, binding);
}

fn preset_css_classes(preset_value: Preset) -> Option<[&'static str; 2]> {
    match preset_value {
        Preset::Disabled => Some(["red", "bold"]),
        Preset::Enabled => Some(["green", "bold"]),
        Preset::Ignore => Some(["yellow", "bold"]),
        _ => None,
    }
}

pub(super) fn get_custom_factoy(prop: &UnitProperty) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(factory_setup);
    let name = prop.name.clone();
    factory.connect_bind(move |_factory, object| {
        let (inscription, unit_binding) = factory_bind_pre!(object);
        let unit = unit_binding.unit();

        inscription.set_text(Some(&name));
        display_inactive!(inscription, unit);
    });

    factory
}
