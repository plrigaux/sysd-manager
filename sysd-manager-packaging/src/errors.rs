#![allow(dead_code)]

use sysd_build_tool::errors::ToolError;

#[derive(Debug)]
pub(crate) enum PackageError {
    Custom(&'static str),
    IoError(std::io::Error),
    CargoNotFound,
    Rpm(rpm::Error),
    NoOption,
    ToolError(ToolError),
    Toml(toml::de::Error),
}

impl From<std::io::Error> for PackageError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<rpm::Error> for PackageError {
    fn from(value: rpm::Error) -> Self {
        Self::Rpm(value)
    }
}

impl From<ToolError> for PackageError {
    fn from(value: ToolError) -> Self {
        Self::ToolError(value)
    }
}

impl From<&'static str> for PackageError {
    fn from(value: &'static str) -> Self {
        Self::Custom(value)
    }
}

impl From<toml::de::Error> for PackageError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}
