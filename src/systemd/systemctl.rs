use super::SystemdErrors;
use std::process::Command;

// Takes the unit pathname of a service and enables it via dbus.
/// If dbus replies with `[Bool(true), Array([], "(sss)")]`, the service is already enabled.
pub fn enable_unit_files_path(unit: &str) -> Result<String, SystemdErrors> {
    let command_output = Command::new("systemctl").arg("enable").arg(unit).output();
    dis_en_able_processing(command_output, "enable")
}

pub fn disable_unit_files_path(unit: &str) -> Result<String, SystemdErrors> {
    let command_output = Command::new("systemctl").arg("disable").arg(unit).output();
    dis_en_able_processing(command_output, "disable")
}

fn dis_en_able_processing(
    command_output: Result<std::process::Output, std::io::Error>,
    action: &str,
) -> Result<String, SystemdErrors> {
    match command_output {
        Ok(output) => {
            let stderr = String::from_utf8(output.stderr)?;
            if output.status.success() {
                Ok(stderr)
            } else {
                Err(SystemdErrors::SystemCtlError(stderr))
            }
        }
        Err(error) => {
            eprintln!("systemctl {} error {}", action, error);
            Err(error.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::systemd::systemctl::*;

    #[test]
    fn test_enable_unit_files_path() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = enable_unit_files_path(file1);
        println!("Status: {:?}", status.unwrap());
    }

    #[test]
    fn test_disable_unit_files_path() {
        //let file1: &str = "/etc/systemd/system/jackett.service";
        let file1: &str = "jackett.service";

        let status = disable_unit_files_path(file1);
        println!("Status: {:?}", status.unwrap());
    }
}
