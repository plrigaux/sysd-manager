use log::info;

use super::SystemdErrors;

#[derive(Clone, Debug)]
pub struct Analyze {
    pub time: u32,
    pub service: String,
}

/// Returns the results of `systemd-analyze blame`
pub fn blame() -> Result<Vec<Analyze>, SystemdErrors> {
    let cmd = ["systemd-analyze", "blame"];
    let command_output = match super::commander(&cmd).output() {
        Ok(output) => {
            if *super::IS_FLATPAK_MODE {
                info!("systemd-analyze status: {}", output.status);

                if !output.status.success() {
                    let v = cmd.iter().map(|&s| s.into()).collect();
                    return Err(SystemdErrors::CmdNoFreedesktopFlatpakPermission(v));
                }
            }

            output.stdout
        }
        Err(e) => {
            let warn_message = format!("Can't call systemd-analyze:  {:?}", e);
            match super::commander_error_handling(&warn_message, e) {
                Ok(_) => return Ok(vec![]),
                Err(e) => return Err(e),
            }
        }
    };

    let collection = String::from_utf8(command_output)
        .expect("from_utf8 failed")
        .lines()
        .rev()
        .map(|x| {
            let mut iterator = x.trim().split_whitespace();
            Analyze {
                time: parse_time(iterator.next().unwrap()),
                service: String::from(iterator.next().unwrap()),
            }
        })
        .collect::<Vec<Analyze>>();

    Ok(collection)
}

fn parse_time(input: &str) -> u32 {
    if input.ends_with("ms") {
        input[0..input.len() - 2].parse::<u32>().unwrap_or(0)
    } else if input.ends_with('s') {
        (input[0..input.len() - 1].parse::<f32>().unwrap_or(0f32) * 1000f32) as u32
    } else if input.ends_with("min") {
        input[0..input.len() - 3].parse::<u32>().unwrap_or(0) * 3600000
    } else {
        0u32
    }
}
