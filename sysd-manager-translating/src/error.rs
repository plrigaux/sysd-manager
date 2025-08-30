use std::{ffi::OsString, process::Command};

#[derive(Debug)]
#[allow(dead_code)]
pub enum TransError {
    IoError(std::io::Error),
    Command(
        OsString,
        Vec<OsString>,
        Vec<(OsString, Option<OsString>)>,
        std::io::Error,
    ),
    BoxError(Box<dyn std::error::Error>),
    LanguageNotSet,
    PathNotExist(String),
    PathNotDIR(String),
}
impl TransError {
    pub(crate) fn create_command_error(command: Command, error: std::io::Error) -> Self {
        let program = command.get_program().to_os_string();
        let envs: Vec<(OsString, Option<OsString>)> = command
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
            .collect();
        let arg: Vec<OsString> = command.get_args().map(|s| s.to_os_string()).collect();

        TransError::Command(program, arg, envs, error)
    }
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
