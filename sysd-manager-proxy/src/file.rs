use base::enums::UnitDBusLevel;
use tracing::info;

pub async fn create_drop_in(
    dbus: u8,
    runtime: bool,
    unit_name: &str,
    file_name: &str,
    content: &str,
) {
    let level: UnitDBusLevel = dbus.into();
    info!(
        "Creating Drop-in: unit {unit_name:?} runtime {runtime:?}, file_name {file_name:?}, bus {level:?}"
    )
}
