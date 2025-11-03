use zvariant::Value;

use super::*;

pub const TEST_SERVICE: &str = "tiny_daemon.service";

fn init() {
    let _ = env_logger::builder()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[ignore = "need a connection to a service"]
#[test]
fn stop_service_test() -> Result<(), SystemdErrors> {
    stop_unit(UnitDBusLevel::System, TEST_SERVICE, StartStopMode::Fail)?;
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_get_unit_file_state() {
    init();
    let file1: &str = TEST_SERVICE;

    let status = get_unit_file_state(UnitDBusLevel::System, file1);
    debug!("Status: {status:?}");
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_list_unit_files_system() -> Result<(), SystemdErrors> {
    init();

    let level = UnitDBusLevel::System;
    let connection = get_connection(level).await?;
    let unit_files = list_unit_files_async(connection, level).await?;

    info!("Unit file returned {}", unit_files.len());

    for (idx, unit_file) in unit_files.iter().enumerate() {
        debug!("{idx} - {}", unit_file.full_name);
        debug!("{}", unit_file.path);
    }

    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_list_unit_files_system_raw() -> Result<(), SystemdErrors> {
    init();

    let level = UnitDBusLevel::System;
    let connection = get_connection(level).await?;
    let message = connection
        .call_method(
            Some(DESTINATION_SYSTEMD),
            PATH_SYSTEMD,
            Some(INTERFACE_SYSTEMD_MANAGER),
            METHOD_LIST_UNIT_FILES,
            &(),
        )
        .await?;

    let body = message.body();

    let array: Vec<LUnitFiles> = body.deserialize()?;

    for (idx, unit_file) in array.iter().enumerate() {
        debug!(
            "{idx} - {} - {}",
            unit_file.enablement_status, unit_file.primary_unit_name
        );
    }

    Ok(())
}

/* fn list_unit_files_user_test(level: UnitDBusLevel) -> Result<Vec<SystemdUnitFile>, SystemdErrors> {
    let units = list_unit_files(&get_connection(level)?, level)?;

    info!("Unit file returned {}", units.len());

    Ok(units)
}

#[ignore = "need a connection to a service"]
#[test]
fn test_list_unit_files_user() -> Result<(), SystemdErrors> {
    init();
    let units = list_unit_files_user_test(UnitDBusLevel::UserSession)?;

    info!("Unit file returned {}", units.len());
    let serv = units.iter().find(|ud| ud.full_name == TEST_SERVICE);

    debug!("{serv:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_list_unit_files_system() -> Result<(), SystemdErrors> {
    init();
    let units = list_unit_files_user_test(UnitDBusLevel::System)?;

    let serv = units.iter().find(|ud| ud.full_name == TEST_SERVICE);

    debug!("{serv:#?}");
    Ok(())
} */

/*
#[ignore = "need a connection to a service"]
#[test]
fn test_list_units() -> Result<(), SystemdErrors> {
    let units = list_units_description(
        &get_connection(UnitDBusLevel::System)?,
        UnitDBusLevel::System,
    )?;

    let serv = units.get(TEST_SERVICE);
    debug!("{:#?}", serv);
    Ok(())
} */

#[ignore = "need a connection to a service"]
#[test]
pub fn test_get_unit_path() -> Result<(), SystemdErrors> {
    init();
    let unit_file: &str = "tiny_daemon.service";

    let connection = get_blocking_connection(UnitDBusLevel::System)?;

    let message = connection.call_method(
        Some(DESTINATION_SYSTEMD),
        PATH_SYSTEMD,
        Some(INTERFACE_SYSTEMD_MANAGER),
        "GetUnit",
        &(unit_file),
    )?;

    info!("message {message:?}");

    let body = message.body();

    let z: zvariant::ObjectPath = body.deserialize()?;
    //let z :String = body.deserialize()?;

    info!("obj {:?}", z.as_str());

    /*         let body = message.body();

    let des = body.deserialize();

    println!("{:#?}", des); */
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
pub fn test_fetch_system_unit_info() -> Result<(), SystemdErrors> {
    init();

    let btree_map = fetch_system_unit_info(
        UnitDBusLevel::System,
        "/org/freedesktop/systemd1/unit/tiny_5fdaemon_2eservice",
        UnitType::Service,
    )?;

    debug!("ALL PARAM: {btree_map:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_enable_unit_files() -> Result<(), SystemdErrors> {
    init();
    let _res = enable_unit_files(
        UnitDBusLevel::System,
        &[TEST_SERVICE],
        DisEnableFlags::SD_SYSTEMD_UNIT_FORCE,
    )?;

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_disable_unit_files() -> Result<(), SystemdErrors> {
    init();
    let _res = disable_unit_files(
        UnitDBusLevel::System,
        &[TEST_SERVICE],
        DisEnableFlags::empty(),
    )?;

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_info() -> Result<(), SystemdErrors> {
    init();

    let path = unit_dbus_path_from_name(TEST_SERVICE);

    println!("unit {TEST_SERVICE} Path {path}");
    let map = fetch_system_unit_info(UnitDBusLevel::System, &path, UnitType::Service)?;

    println!("{map:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_system_info() -> Result<(), SystemdErrors> {
    init();

    let map = fetch_system_info(UnitDBusLevel::System)?;

    info!("{map:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit() -> Result<(), SystemdErrors> {
    init();

    let unit = fetch_unit(UnitDBusLevel::System, TEST_SERVICE)?;

    info!("{unit:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit_user_session() -> Result<(), SystemdErrors> {
    init();

    let unit = fetch_unit(UnitDBusLevel::UserSession, TEST_SERVICE)?;

    info!("{unit:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit_wrong_bus() -> Result<(), SystemdErrors> {
    init();

    let unit = fetch_unit(UnitDBusLevel::UserSession, TEST_SERVICE)?;

    info!("{}", unit.debug());
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit_dependencies() -> Result<(), SystemdErrors> {
    init();

    let path = unit_dbus_path_from_name(TEST_SERVICE);
    let res = unit_get_dependencies(
        UnitDBusLevel::System,
        TEST_SERVICE,
        &path,
        DependencyType::Forward,
        false,
    );

    info!("{:#?}", res.unwrap());
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit_reverse_dependencies() -> Result<(), SystemdErrors> {
    init();

    let path = unit_dbus_path_from_name(TEST_SERVICE);
    let res = unit_get_dependencies(
        UnitDBusLevel::System,
        TEST_SERVICE,
        &path,
        DependencyType::Reverse,
        false,
    );

    info!("{res:#?}");
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_fetch_unit_fail_wrong_name() -> Result<(), SystemdErrors> {
    init();

    let fake = format!("{TEST_SERVICE}_fake");
    match fetch_unit(UnitDBusLevel::System, &fake) {
        Ok(_) => todo!(),
        Err(e) => {
            warn!("{e:?}");
            if let SystemdErrors::ZNoSuchUnit(_method, _message) = e {
                Ok(())
            } else {
                Err(SystemdErrors::Custom("Wrong expected Error".to_owned()))
            }
        }
    }
}

#[test]
fn test_name_convertion() {
    let tests = [
        ("tiny_daemon.service", "tiny_5fdaemon_2eservice"),
        ("-.mount", "_2d_2emount"),
        //("sys-devices-pci0000:00-0000:00:1d.0-0000:3d:00.0-nvme-nvme0-nvme0n1-nvme0n1p1.device", "sys_2ddevices_2dpci0000_3a00_2d0000_3a00_3a1d_2e0_2d0000_3a3d_3a00_2e0_2dnvme_2dnvme0_2dnvme0n1_2dnvme0n1p1_2edevice"),
        ("1first", "_31first"),
    ];

    for (origin, expected) in tests {
        let convertion = bus_label_escape(origin);
        assert_eq!(convertion, expected);
    }
}

#[ignore = "need a connection to a service"]
#[test]
fn test_get_unit_processes() -> Result<(), SystemdErrors> {
    let unit_file: &str = "system.slice";

    let list = retreive_unit_processes(UnitDBusLevel::System, unit_file)?;

    for up in list {
        println!("{up:#?}")
    }

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_get_unit_active_state() -> Result<(), SystemdErrors> {
    let unit_object = unit_dbus_path_from_name(TEST_SERVICE);

    println!("path : {unit_object}");
    let state = get_unit_active_state(UnitDBusLevel::System, &unit_object)?;

    println!("state of {TEST_SERVICE} is {state:?}");

    Ok(())
}

async fn get_unit_list_test(level: UnitDBusLevel) -> Result<Vec<LUnit>, SystemdErrors> {
    let connection = get_connection(level).await?;

    let r = list_units_list_async(connection).await?;

    info!("Returned units count: {}", r.len());

    Ok(r)
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_unit_list_system() -> Result<(), SystemdErrors> {
    init();
    let _map = get_unit_list_test(UnitDBusLevel::System).await?;
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_unit_list_user() -> Result<(), SystemdErrors> {
    init();
    let _map = get_unit_list_test(UnitDBusLevel::UserSession).await?;
    Ok(())
}

async fn get_unit_file_list_test(
    level: UnitDBusLevel,
) -> Result<Vec<SystemdUnitFile>, SystemdErrors> {
    let connection = get_connection(level).await?;

    let r = list_unit_files_async(connection, level).await?;

    info!("Returned units count: {}", r.len());

    Ok(r)
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_unit_file_list_system() -> Result<(), SystemdErrors> {
    init();
    let _map = get_unit_file_list_test(UnitDBusLevel::System).await?;
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_unit_file_list_user() -> Result<(), SystemdErrors> {
    init();
    let _map = get_unit_list_test(UnitDBusLevel::UserSession).await?;
    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_list() -> Result<(), SystemdErrors> {
    init();
    let connection = get_connection(UnitDBusLevel::System).await?;

    let connection2 = get_connection(UnitDBusLevel::UserSession).await?;

    use std::time::Instant;
    let now = Instant::now();
    let t1 = tokio::spawn(list_units_list_async(connection.clone()));
    let t2 = tokio::spawn(list_unit_files_async(connection, UnitDBusLevel::System));
    let t3 = tokio::spawn(list_units_list_async(connection2.clone()));
    let t4 = tokio::spawn(list_unit_files_async(
        connection2,
        UnitDBusLevel::UserSession,
    ));

    let _asdf = tokio::join!(t1, t2, t3, t4);

    let elapsed = now.elapsed();
    println!("Elapsed: {elapsed:.2?}");
    /*        let a = asdf.0;
    let b = asdf.1;

    println!("{:?}", a);
    println!("{:?}", b); */

    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_list2() -> Result<(), SystemdErrors> {
    let connection = get_connection(UnitDBusLevel::System).await?;

    let connection2 = get_connection(UnitDBusLevel::UserSession).await?;

    use std::time::Instant;
    let now = Instant::now();
    let t1 = list_units_list_async(connection.clone());
    let t2 = list_unit_files_async(connection, UnitDBusLevel::System);
    let t3 = list_units_list_async(connection2.clone());
    let t4 = list_unit_files_async(connection2, UnitDBusLevel::UserSession);

    let joined_result = tokio::join!(t1, t2, t3, t4);

    let elapsed = now.elapsed();
    println!("Elapsed: {elapsed:.2?}");

    let r1 = joined_result.0.unwrap();
    let r2 = joined_result.1.unwrap();
    let r3 = joined_result.2.unwrap();
    let r4 = joined_result.3.unwrap();

    println!("System unit description size {}", r1.len());
    println!("System unit file size {}", r2.len());
    println!("Session unit description size {}", r3.len());
    println!("Session unit file size {}", r4.len());

    //check system collision
    /*     for (key, _val) in r1 {
        if r3.contains_key(&key) {
            println!("collision description on key {key}");
        }
    } */

    /*         let a = asdf.0;
           let b = asdf.1;

           println!("{:?}", a);
           println!("{:?}", b);
    */
    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_get_properties() -> Result<(), SystemdErrors> {
    init();
    let connection = get_blocking_connection(UnitDBusLevel::System)?;

    let object_path = unit_dbus_path_from_name(TEST_SERVICE);
    debug!("Unit path: {object_path}");
    let properties_proxy: zbus::blocking::fdo::PropertiesProxy =
        fdo::PropertiesProxy::builder(&connection)
            .destination(DESTINATION_SYSTEMD)?
            .path(object_path)?
            // .interface(INTERFACE_SYSTEMD_UNIT)?
            //  .interface(UnitType::Service.interface())?
            .build()?;

    let unit_type = UnitType::Service;
    let unit_interface = unit_type.interface();

    //let unit_interface_name = InterfaceName::try_from(INTERFACE_SYSTEMD_UNIT).unwrap();
    let unit_interface_name = InterfaceName::try_from(INTERFACE_SYSTEMD_UNIT).unwrap();

    let mut unit_properties: HashMap<String, OwnedValue> =
        properties_proxy.get_all(unit_interface_name)?;

    let interface_name = InterfaceName::try_from(unit_interface).unwrap();

    let properties: HashMap<String, OwnedValue> = properties_proxy.get_all(interface_name)?;

    info!("Properties size {}", properties.len());

    info!("Unit Properties size {}", unit_properties.len());

    for k in properties.into_keys() {
        unit_properties.remove(&k);
    }

    info!("Unit Properties size {}", unit_properties.len());

    /*      for (k, v) in unit_properties {
        info!("{k} {:?}", v);
    } */

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_kill_unit() -> Result<(), SystemdErrors> {
    init();
    let unit_name: &str = TEST_SERVICE;

    kill_unit(UnitDBusLevel::System, unit_name, KillWho::Main, 1)
}

#[ignore = "need a connection to a service"]
#[test]
fn test_queue_signal_unit() -> Result<(), SystemdErrors> {
    init();
    let unit_name: &str = TEST_SERVICE;
    let val: i32 = libc::SIGRTMIN();
    let val2 = libc::SIGRTMAX();

    println!("{val} {val2}");

    queue_signal_unit(UnitDBusLevel::System, unit_name, KillWho::Main, 9, 0)
}

#[ignore = "need a connection to a service"]
#[test]
pub(super) fn test_unit_clean() -> Result<(), SystemdErrors> {
    let handle_answer = |_method: &str, _return_message: &Message| {
        info!("Clean Unit SUCCESS");

        Ok(())
    };
    let path = unit_dbus_path_from_name(TEST_SERVICE);
    let what = ["logs"];

    send_unit_message(
        UnitDBusLevel::System,
        "Clean",
        &(&what),
        handle_answer,
        &path,
    )
}

fn send_unit_message<T, U>(
    level: UnitDBusLevel,
    method: &str,
    body: &T,
    handler: impl Fn(&str, &Message) -> Result<U, SystemdErrors>,
    path: &str,
) -> Result<U, SystemdErrors>
where
    T: serde::ser::Serialize + DynamicType,
    U: std::fmt::Debug,
{
    let message = Message::method_call(path, method)?
        .with_flags(Flags::AllowInteractiveAuth)?
        .destination(DESTINATION_SYSTEMD)?
        .interface(INTERFACE_SYSTEMD_UNIT)?
        .build(body)?;

    let connection = get_blocking_connection(level)?;

    connection.send(&message)?;

    let message_it = MessageIterator::from(connection);

    for message_res in message_it {
        debug!("Message response {message_res:?}");
        let return_message = message_res?;

        match return_message.message_type() {
            zbus::message::Type::MethodReturn => {
                info!("{method} Response");
                let result = handler(method, &return_message);
                return result;
            }
            zbus::message::Type::MethodCall => {
                warn!("Not supposed to happen: {return_message:?}");
                break;
            }
            zbus::message::Type::Error => {
                let error = zbus::Error::from(return_message);
                {
                    match error {
                        zbus::Error::MethodError(
                            ref owned_error_name,
                            ref details,
                            ref message,
                        ) => {
                            warn!(
                                "Method error: {}\nDetails: {}\n{:?}",
                                owned_error_name.as_str(),
                                details.as_ref().map(|s| s.as_str()).unwrap_or_default(),
                                message
                            )
                        }
                        _ => warn!("Bus error: {error:?}"),
                    }
                }
                return Err(SystemdErrors::from(error));
            }
            zbus::message::Type::Signal => {
                info!("Signal: {return_message:?}");
                continue;
            }
        }
    }

    let msg = format!("{method:?} ????, response supposed to be Unreachable");
    warn!("{msg}");
    Err(SystemdErrors::Malformed(
        msg,
        "sequences of messages".to_owned(),
    ))
}

#[ignore = "need a connection to a service"]
#[test]
fn test_mask_unit_file() -> Result<(), SystemdErrors> {
    init();
    let unit_name: &str = TEST_SERVICE;

    mask_unit_files(UnitDBusLevel::System, &[unit_name], false, false)?;

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_mask_unit_file2() -> Result<(), SystemdErrors> {
    init();
    let unit_file_name: &str = "/etc/systemd/system/tiny_daemon.service";

    mask_unit_files(UnitDBusLevel::System, &[unit_file_name], false, false)?;

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_unmask_unit_file() -> Result<(), SystemdErrors> {
    init();
    let unit_name: &str = TEST_SERVICE;

    unmask_unit_files(UnitDBusLevel::System, &[unit_name], false)?;

    Ok(())
}

#[ignore = "need a connection to a service"]
#[test]
fn test_introspect() -> Result<(), SystemdErrors> {
    init();

    let result: Result<(), SystemdErrors> = {
        let connection = get_blocking_connection(UnitDBusLevel::System)?;
        info!("Connect");

        let message = connection.call_method(
            Some(DESTINATION_SYSTEMD),
            //"/org/freedesktop/systemd1/unit/avahi_2ddaemon_2eservice",
            //"/",
            //"/org/freedesktop/systemd1/unit",
            "/org/freedesktop/systemd1/archlinux_2dkeyring_2dwkd_2dsync_2etimer",
            //  Some("org.freedesktop.DBus.Properties"),
            Some("org.freedesktop.DBus.Introspectable"),
            "Introspect",
            // &(UnitType::Service.interface()),
            &(),
        )?;

        info!("message {message:?}");

        let body = message.body();

        info!("signature {:?}", body.signature());

        //let z: Vec<(String, OwnedValue)> = body.deserialize()?;

        let z: String = body.deserialize()?;
        //let z :String = body.deserialize()?;

        info!("obj {:?}", z);

        /*         let body = message.body();

        let des = body.deserialize();

        println!("{:#?}", des); */

        /*     match get_unit_file_state(level, unit_primary_name) {
            Ok(unit_file_status) => unit.set_enable_status(unit_file_status as u8),
            Err(err) => warn!("Fail to get unit file state : {:?}", err),
        } */
        Ok(())
    };

    if let Err(ref e) = result {
        warn!("ASTFGSDFGD {e:?}");
    }
    result
}

#[ignore = "need a connection to a service"]
#[test]
fn test_introspect2() -> Result<(), SystemdErrors> {
    init();

    fn sub() -> Result<(), SystemdErrors> {
        let connection = get_blocking_connection(UnitDBusLevel::System)?;

        let proxy = Proxy::new(
            &connection,
            DESTINATION_SYSTEMD,
            "/org/freedesktop/systemd1/unit/archlinux_2dkeyring_2dwkd_2dsync_2etimer",
            "org.freedesktop.DBus.Introspectable",
        )?;

        info!("Proxy {proxy:?}");

        let xml = proxy.introspect()?;

        let root_node = zbus_xml::Node::from_reader(xml.as_bytes())?;

        for int in root_node.interfaces() {
            info!("Interface {}", int.name());

            for prop in int.properties() {
                info!("\tProp {} {:?}", prop.name(), prop.ty().to_string());
            }
        }

        Ok(())
    }

    sub().map_err(|e| {
        warn!("ASTFGSDFGD {e:?}");
        e
    })
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_introspect3() -> Result<(), SystemdErrors> {
    init();

    let map = fetch_unit_interface_properties().await?;

    for (k, v) in map.iter() {
        info!("{k}\t{}", v.len());
    }

    Ok(())
}

#[ignore = "need a connection to a service"]
#[tokio::test]
async fn test_get_properties2() -> Result<(), SystemdErrors> {
    init();

    let connection = get_connection(UnitDBusLevel::System).await?;

    let object_path = unit_dbus_path_from_name(TEST_SERVICE);
    let message = connection
        .call_method(
            Some(DESTINATION_SYSTEMD),
            //"/org/freedesktop/systemd1/unit/avahi_2ddaemon_2eservice",
            //"/",
            //"/org/freedesktop/systemd1/unit",
            // "/org/freedesktop/systemd1/archlinux_2dkeyring_2dwkd_2dsync_2etimer",
            object_path,
            //  Some("org.freedesktop.DBus.Properties"),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            // &(UnitType::Service.interface()),
            //      &("org.freedesktop.systemd1.Unit", "Id", "ConditionTimestamp"),
            &("org.freedesktop.systemd1.Unit", "ConditionTimestamp"),
        )
        .await?;

    let body = message.body();

    info!("signature {:?}", body.signature().to_string());

    //let z: Vec<(String, OwnedValue)> = body.deserialize()?;

    //let z: OwnedValue = body.deserialize()?;
    let z: Value = body.deserialize()?;
    //let z :String = body.deserialize()?;

    info!("obj {:?}", z);

    Ok(())
}
