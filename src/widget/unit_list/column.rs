use std::borrow::Cow;

use glib::GString;

use crate::{
    consts::COL_ACTIVE,
    widget::unit_list::{COL_ID_UNIT, COL_ID_UNIT_FULL},
};

#[derive(Debug)]
pub enum SysdColumn {
    Name,
    FullName,
    Bus,
    Type,
    State,
    Preset,
    Load,
    Active,
    Sub,
    Description,
    Custom(String, Option<String>),
}

impl SysdColumn {
    pub fn id<'a>(&self) -> Cow<'a, str> {
        match self {
            SysdColumn::Name => Cow::from(COL_ID_UNIT),
            SysdColumn::FullName => Cow::from(COL_ID_UNIT_FULL),
            SysdColumn::Bus => Cow::from("sysdm-bus"),
            SysdColumn::Type => Cow::from("sysdm-type"),
            SysdColumn::State => Cow::from("sysdm-state"),
            SysdColumn::Preset => Cow::from("sysdm-preset"),
            SysdColumn::Load => Cow::from("sysdm-load"),
            SysdColumn::Active => Cow::from(COL_ACTIVE),
            SysdColumn::Sub => Cow::from("sysdm-sub"),
            SysdColumn::Description => Cow::from("sysdm-description"),
            SysdColumn::Custom(id, _) => Cow::Owned(id.to_owned()),
        }
    }

    pub fn property_type(&self) -> &Option<String> {
        match self {
            SysdColumn::Custom(_, p) => p,
            _ => &None,
        }
    }
}

impl From<(GString, Option<String>)> for SysdColumn {
    fn from(value: (GString, Option<String>)) -> Self {
        match value.0.as_str() {
            COL_ID_UNIT => SysdColumn::Name,
            COL_ID_UNIT_FULL => SysdColumn::FullName,
            "sysdm-bus" => SysdColumn::Bus,
            "sysdm-type" => SysdColumn::Type,
            "sysdm-state" => SysdColumn::State,
            "sysdm-preset" => SysdColumn::Preset,
            "sysdm-load" => SysdColumn::Load,
            COL_ACTIVE => SysdColumn::Active,
            "sysdm-sub" => SysdColumn::Bus,
            "sysdm-description" => SysdColumn::Description,
            _ => SysdColumn::Custom(value.0.to_string(), value.1),
        }
    }
}
