use super::{commander_output, SystemdErrors};
use log::error;

const SYSTEMCTL: &str = "systemctl";
const ENABLE: &str = "enable";
const DISABLE: &str = "disable";

// Takes the unit pathname of a service and enables it via dbus.
/// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
pub fn enable_unit_files_path(unit: &str) -> Result<String, SystemdErrors> {
    let command_output = commander_output(&[SYSTEMCTL, ENABLE, unit], None);
    dis_en_able_processing(command_output, ENABLE)
}

pub fn disable_unit_files_path(unit: &str) -> Result<String, SystemdErrors> {
    let command_output = commander_output(&[SYSTEMCTL, DISABLE, unit], None);
    dis_en_able_processing(command_output, DISABLE)
}

fn dis_en_able_processing(
    command_output: Result<std::process::Output, SystemdErrors>,
    action: &str,
) -> Result<String, SystemdErrors> {
    match command_output {
        Ok(output) => {
            let error_msg = String::from_utf8(output.stderr)?;
            if output.status.success() {
                Ok(error_msg)
            } else {
                Err(SystemdErrors::SystemCtlError(error_msg))
            }
        }
        Err(error) => {
            error!("{} {} error {:?}", SYSTEMCTL, action, error);
            Err(error)
        }
    }
}
/*
#[cfg(test)]
mod tests {
    use log::debug;

    use crate::systemd::systemctl::*;

    #[test]
    fn test_enable_unit_files_path() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = enable_unit_files_path(file1);
        debug!("Status: {:?}", status.unwrap());
    }

    #[test]
    fn test_disable_unit_files_path() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = disable_unit_files_path(file1);
        debug!("Status: {:?}", status.unwrap());
    }
}
 */
