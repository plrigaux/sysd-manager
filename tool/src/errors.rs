#[derive(Debug)]
pub enum ToolError {
    IoError(std::io::Error),
    FileNotFound,
}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}
