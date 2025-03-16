use crate::widget::unit_list::UnitListPanel;

use super::*;

macro_rules! factory_setup {
    ($item_obj:expr) => {{
        let item = $item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        let inscription = gtk::Inscription::builder()
            .xalign(0.0)
            //.wrap_mode(gtk::pango::WrapMode::None)
            .wrap_mode(gtk::pango::WrapMode::Char)
            .build();
        item.set_child(Some(&inscription));
        inscription
    }};
}

macro_rules! downcast_list_item {
    ($item_obj:expr) => {{
        let item = $item_obj
            .downcast_ref::<gtk::ListItem>()
            .expect("item.downcast_ref::<gtk::ListItem>()");
        item
    }};
}

macro_rules! downcast_unit_binding {
    ($item_obj:expr) => {{
        let list_item = downcast_list_item!($item_obj);

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
    ($item_obj:expr) => {{
        let item = downcast_list_item!($item_obj);
        let child = item
            .child()
            .and_downcast::<gtk::Inscription>()
            .expect("item.downcast_ref::<gtk::Inscription>()");
        let unit_binding = item
            .item()
            .and_downcast::<UnitBinding>()
            .expect("item.downcast_ref::<gtk::UnitBinding>()");
        (child, unit_binding)
    }};
}

macro_rules! factory_bind {
    ($item_obj:expr, $func:ident) => {{
        let (child, unit_binding) = factory_bind_pre!($item_obj);
        let unit = unit_binding.unit();
        let text = unit.$func();
        child.set_text(Some(&text));
        (child, unit, unit_binding)
    }};
}

pub fn setup_factories(
    unit_list: UnitListPanel,
    column_view_column_map: &HashMap<glib::GString, gtk::ColumnViewColumn>,
) {
    let fac_unit_name = SignalListItemFactory::new();

    fac_unit_name.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_unit_name.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind!(object, display_name);
            unit_list.imp().display_inactive(inscription, &unit);
        });
    }

    let fac_unit_type = SignalListItemFactory::new();

    fac_unit_type.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_unit_type.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind!(object, unit_type);
            unit_list.imp().display_inactive(inscription, &unit);
        });
    }

    let fac_bus = SignalListItemFactory::new();

    fac_bus.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_bus.connect_bind(move |_factory, object| {
            let (inscription, unit, _unit_binding) = factory_bind!(object, dbus_level_str);
            unit_list.imp().display_inactive(inscription, &unit);
        });
    }

    let fac_enable_status = SignalListItemFactory::new();

    fac_enable_status.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_enable_status.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind!(object, sub_state);

            let binding = unit
                .bind_property("enable_status", &inscription, "text")
                .transform_to(|_, status: u8| {
                    let estatus: EnablementStatus = status.into();
                    let str = estatus.to_string();
                    Some(str)
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_STATUS_TEXT, binding);

            let is_dark = unit_list.imp().is_dark.get();
            let binding = unit
                .bind_property("enable_status", &inscription, "attributes")
                .transform_to_with_values(move |_s, value| {
                    let value = match value.get::<String>() {
                        Ok(v) => v,
                        Err(err) => {
                            warn!("The variant needs to be of type `String`. {:?}", err);
                            return None;
                        }
                    };

                    let attribute_list = if let Some(first_char) = value.chars().next() {
                        match first_char {
                            'm' | 'd' | 'b' => {
                                Some(UnitListPanelImp::highlight_attrlist(red(is_dark)))
                            }

                            'e' | 'a' => Some(UnitListPanelImp::highlight_attrlist(green(is_dark))),

                            _ => None,
                        }
                    } else {
                        None
                    };

                    attribute_list.map(|al| al.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_STATUS_ATTR, binding);

            let status_code: EnablementStatus = unit.enable_status().into();
            let status_code_str = status_code.as_str();

            inscription.set_text(Some(status_code_str));
            inscription.set_tooltip_markup(Some(status_code.tooltip_info()));

            let attrs = if let Some(first_char) = status_code_str.chars().next() {
                match first_char {
                    'm' | 'd' | 'b' => {
                        //"disabled"
                        unit_list.imp().highlight_red.borrow().copy()
                    }

                    'e' | 'a' => {
                        //"enabled" or "alias"
                        unit_list.imp().highlight_green.borrow().copy()
                    }

                    _ => None,
                }
            } else {
                None
            };

            if attrs.is_some() {
                inscription.set_attributes(attrs.as_ref());
            } else {
                unit_list.imp().display_inactive(inscription, &unit);
            }
        });
    }

    factory_connect_unbind!(
        fac_enable_status,
        BIND_ENABLE_STATUS_TEXT,
        BIND_ENABLE_STATUS_ATTR
    );

    let fac_preset = SignalListItemFactory::new();

    fac_preset.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_preset.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind!(object, preset);

            let binding = unit.bind_property("preset", &inscription, "text").build();
            unit_binding.set_binding(BIND_ENABLE_PRESET_TEXT, binding);

            let is_dark = unit_list.imp().is_dark.get();
            let binding = unit
                .bind_property("preset", &inscription, "attributes")
                .transform_to_with_values(move |_s, value| {
                    let value = match value.get::<String>() {
                        Ok(v) => v,
                        Err(err) => {
                            warn!("The variant needs to be of type `String`. {:?}", err);
                            return None;
                        }
                    };

                    let attribute_list = if let Some(first_char) = value.chars().next() {
                        match first_char {
                            //"disabled"
                            'd' => Some(UnitListPanelImp::highlight_attrlist(red(is_dark))),
                            // "enabled"
                            'e' => Some(UnitListPanelImp::highlight_attrlist(green(is_dark))),
                            // "ignored"
                            'i' => Some(UnitListPanelImp::highlight_attrlist(yellow(is_dark))),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    attribute_list.map(|al| al.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_PRESET_ATTR, binding);

            let attrs = if let Some(first_char) = unit.preset().chars().next() {
                match first_char {
                    //"disabled"
                    'd' => unit_list.imp().highlight_red.borrow().copy(),
                    // "enabled"
                    'e' => unit_list.imp().highlight_green.borrow().copy(),
                    // "ignored"
                    'i' => unit_list.imp().highlight_yellow.borrow().copy(),
                    _ => None,
                }
            } else {
                None
            };

            if attrs.is_some() {
                inscription.set_attributes(attrs.as_ref());
            } else {
                unit_list.imp().display_inactive(inscription, &unit);
            }
        });
    }

    factory_connect_unbind!(fac_preset, BIND_ENABLE_PRESET_TEXT, BIND_ENABLE_PRESET_ATTR);

    let fac_load_state = SignalListItemFactory::new();

    fac_load_state.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    {
        let unit_list = unit_list.clone();
        fac_load_state.connect_bind(move |_factory, object| {
            let (inscription, unit, unit_binding) = factory_bind!(object, load_state);

            let binding = unit
                .bind_property("load_state", &inscription, "text")
                .build();
            unit_binding.set_binding(BIND_ENABLE_LOAD_TEXT, binding);

            let is_dark = unit_list.imp().is_dark.get();
            let binding = unit
                .bind_property("preset", &inscription, "attributes")
                .transform_to_with_values(move |_s, value| {
                    let value = match value.get::<String>() {
                        Ok(v) => v,
                        Err(err) => {
                            warn!("The variant needs to be of type `String`. {:?}", err);
                            return None;
                        }
                    };

                    let attribute_list = if let Some(first_char) = value.chars().next() {
                        match first_char {
                            'n' => {
                                //"not-found"
                                let al = UnitListPanelImp::highlight_attrlist(yellow(is_dark));
                                Some(al)
                            }
                            'b' | 'e' | 'm' => {
                                // "bad-setting", "error", "masked"
                                let al = UnitListPanelImp::highlight_attrlist(red(is_dark));
                                Some(al)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };

                    attribute_list.map(|al| al.to_value())
                })
                .build();

            unit_binding.set_binding(BIND_ENABLE_LOAD_ATTR, binding);

            let load_state = unit.load_state();

            if let Some(first_char) = load_state.chars().next() {
                match first_char {
                    'n' => {
                        //"not-found"
                        let attribute_list = unit_list.imp().highlight_yellow.borrow();
                        inscription.set_attributes(Some(&attribute_list));
                    }
                    'b' | 'e' | 'm' => {
                        // "bad-setting", "error", "masked"
                        let attribute_list = unit_list.imp().highlight_red.borrow();
                        inscription.set_attributes(Some(&attribute_list));
                    }
                    _ => unit_list.imp().display_inactive(inscription, &unit),
                }
            }
        });
    }

    factory_connect_unbind!(fac_load_state, BIND_ENABLE_LOAD_TEXT, BIND_ENABLE_LOAD_ATTR);

    let fac_active = SignalListItemFactory::new();

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
            .bind_property("active_state_num", &icon_image, "icon-name")
            .transform_to(|_, state: u8| {
                let state: ActiveState = state.into();
                icon_name!(state)
            })
            .build();

        unit_binding.set_binding(BIND_ENABLE_ACTIVE_ICON, binding);

        if state.is_inactive() {
            icon_image.add_css_class("grey");
        } else {
            icon_image.remove_css_class("grey");
        }
    });

    factory_connect_unbind!(fac_active, BIND_ENABLE_ACTIVE_ICON);

    let fac_sub_state = SignalListItemFactory::new();

    fac_sub_state.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    fac_sub_state.connect_bind(|_factory, object| {
        let (child, unit, unit_binding) = factory_bind!(object, sub_state);
        let binding = unit.bind_property("sub_state", &child, "text").build();
        unit_binding.set_binding(BIND_SUB_STATE_TEXT, binding);
    });

    factory_connect_unbind!(fac_sub_state, BIND_SUB_STATE_TEXT);

    let fac_descrition = SignalListItemFactory::new();

    fac_descrition.connect_setup(|_factory, object| {
        factory_setup!(object);
    });

    fac_descrition.connect_bind(|_factory, object| {
        let (child, unit, unit_binding) = factory_bind!(object, description);
        let binding = unit.bind_property("description", &child, "text").build();
        unit_binding.set_binding(BIND_DESCRIPTION_TEXT, binding);
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
            info!(
                "Column width {:?} {}",
                cvc.id().unwrap_or_default(),
                cvc.fixed_width()
            )
        });
    }
}
