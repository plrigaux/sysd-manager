#[derive(Debug)]
#[allow(dead_code)]
pub enum TransError {
    IoError(std::io::Error),
    BoxError(Box<dyn std::error::Error>),
    LanguageNotSet,
}

impl From<Box<dyn std::error::Error>> for TransError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        TransError::BoxError(value)
    }
}

impl From<std::io::Error> for TransError {
    fn from(value: std::io::Error) -> Self {
        TransError::IoError(value)
    }
}
