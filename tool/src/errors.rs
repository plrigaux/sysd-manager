use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum ToolError {
    IoError(std::io::Error),
    FileNotFound,
    Utf8Error(FromUtf8Error),
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
