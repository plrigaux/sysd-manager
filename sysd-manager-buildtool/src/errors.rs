use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum ToolError {
    IoError(std::io::Error),
    FileNotFound,
    Utf8Error(FromUtf8Error),
    Rpm(rpm::Error),
    NoOption,
    CargoNotFound,
    Toml(toml::de::Error),
    Custom(&'static str),
}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<FromUtf8Error> for ToolError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<rpm::Error> for ToolError {
    fn from(value: rpm::Error) -> Self {
        Self::Rpm(value)
    }
}

impl From<toml::de::Error> for ToolError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}
