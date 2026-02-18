use base::{
    RunMode,
    consts::*,
    proxy::{DisEnAbleUnitFiles, DisEnAbleUnitFilesResponse},
};

use std::{borrow::Cow, env, error::Error};
use tokio::sync::OnceCell;
use tracing::{debug, info, warn};
use zbus::{
    Connection, ObjectServer, connection, interface, message::Header, object_server::SignalEmitter,
};

use crate::{SysDManagerProxy, file, sysdcom};

#[interface(name = "io.github.plrigaux.SysDManager", introspection_docs = true)]
impl SysDManagerProxy {
    pub async fn create_drop_in(
        &mut self,
        #[zbus(header)] header: Header<'_>,
        runtime: bool,
        unit_name: &str,
        file_name: &str,
        content: &str,
    ) -> zbus::fdo::Result<()> {
        //self.
        self.check_autorisation(header).await?;

        //   self.get_all(object_server, connection, header, emitter)
        file::create_drop_in(runtime, unit_name, file_name, content).await
    }

    pub async fn save_file(
        &mut self,
        #[zbus(header)] header: Header<'_>,

        file_path: &str,
        content: &str,
    ) -> zbus::fdo::Result<u64> {
        self.check_autorisation(header).await?;
        file::save(file_path, content).await
    }

    pub async fn my_user_id(
        &mut self,
        #[zbus(header)] header: Header<'_>,
    ) -> zbus::fdo::Result<u32> {
        self.check_autorisation(header).await?;

        let id = unsafe { libc::getegid() };
        info!("ids {}", id);

        Ok(id)
    }

    // "Bye" signal (note: no implementation body).
    #[zbus(signal)]
    async fn bye(signal_emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hello(signal_emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

    // "Quit" method. A method may throw errors.
    async fn quit(
        &self,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        #[zbus(object_server)] _server: &ObjectServer,
    ) -> zbus::fdo::Result<()> {
        let path = hdr.path().unwrap();
        let msg = format!("You are leaving me on the {} path?", path);
        emitter.bye(&msg).await?;

        // Do some asynchronous tasks before quitting..

        Ok(())
    }

    async fn even_ping(
        &mut self,
        #[zbus(header)] header: Header<'_>,
        val: u32,
    ) -> zbus::fdo::Result<u32> {
        info!("even_ping {val}");
        self.check_autorisation(header).await?;
        if val.is_multiple_of(2) {
            Ok(val)
        } else {
            Err(zbus::fdo::Error::Failed(format!("{val} not even!")))
        }
    }

    async fn clean_unit(
        &self,
        #[zbus(header)] header: Header<'_>,

        unit_name: &str,
        what: Vec<&str>,
    ) -> zbus::fdo::Result<()> {
        info!("clean_unit {} {:?}", unit_name, what);

        self.check_autorisation(header).await?;

        let proxy = get_proxy().await?;
        proxy
            .clean_unit(unit_name, &what)
            .await
            .inspect_err(|e| warn!("Error while calling clean_unit on sysdbus proxy: {:?}", e))
    }

    async fn freeze_unit(
        &self,
        #[zbus(header)] header: Header<'_>,
        unit_name: &str,
    ) -> zbus::fdo::Result<()> {
        info!("freeze_unit {}", unit_name);
        self.check_autorisation(header).await?;

        let proxy = get_proxy().await?;
        proxy
            .freeze_unit(unit_name)
            .await
            .inspect_err(|e| warn!("Error while calling freeze_unit on sysdbus proxy: {:?}", e))
    }

    async fn thaw_unit(
        &self,
        #[zbus(header)] header: Header<'_>,
        unit_name: &str,
    ) -> zbus::fdo::Result<()> {
        info!("thaw_unit {}", unit_name);
        self.check_autorisation(header).await?;

        let proxy = get_proxy().await?;
        proxy
            .thaw_unit(unit_name)
            .await
            .inspect_err(|e| warn!("Error while calling thaw_unit on sysdbus proxy: {:?}", e))
    }

    async fn revert_unit_files(
        &self,
        #[zbus(header)] header: Header<'_>,
        file_names: Vec<String>,
    ) -> zbus::fdo::Result<Vec<DisEnAbleUnitFiles>> {
        info!("Revert_unit_files  {:?}", file_names);

        let proxy: &sysdcom::SysDManagerComLinkProxy<'_> = get_proxy().await?;

        debug!("Proxy {:?}", proxy);
        self.check_autorisation(header).await?;
        debug!("Polkit autorized");
        match proxy.revert_unit_files(&file_names).await {
            Ok(vec) => {
                info!("revert_unit_files {:?} --> {:?}", file_names, vec);
                Ok(vec)
            }
            Err(err) => {
                warn!(
                    "Error while calling revert_unit_files on sysdbus proxy: {:?}",
                    err
                );
                Err(err)
            }
        }
    }

    async fn reload(&self, #[zbus(header)] header: Header<'_>) -> zbus::fdo::Result<()> {
        info!("Reload");
        let proxy: &sysdcom::SysDManagerComLinkProxy<'_> = get_proxy().await?;
        self.check_autorisation(header).await?;
        debug!("Polkit autorized");
        proxy
            .reload()
            .await
            .inspect_err(|e| warn!("Error while calling reload on sysdbus proxy: {:?}", e))
    }

    async fn enable_unit_files_with_flags(
        &self,
        #[zbus(header)] header: Header<'_>,
        unit_files: Vec<&str>,
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse> {
        info!(
            "enable_unit_files_with_flags {:?} flags {}",
            unit_files, flags
        );
        self.check_autorisation(header).await?;

        let proxy = get_proxy().await?;
        proxy
            .enable_unit_files_with_flags(&unit_files, flags)
            .await
            .inspect_err(|e| {
                warn!(
                    "Error while calling disable_unit_files_with_flags on sysdbus proxy: {:?}",
                    e
                )
            })
    }

    async fn disable_unit_files_with_flags(
        &self,
        #[zbus(header)] header: Header<'_>,
        unit_files: Vec<&str>,
        flags: u64,
    ) -> zbus::fdo::Result<DisEnAbleUnitFilesResponse> {
        info!(
            "disable_unit_files_with_flags {:?} flags {}",
            unit_files, flags
        );
        self.check_autorisation(header).await?;

        let proxy = get_proxy().await?;
        proxy
            .disable_unit_files_with_flags_and_install_info(&unit_files, flags)
            .await
            .inspect_err(|e| {
                warn!(
                    "Error while calling disable_unit_files_with_flags on sysdbus proxy: {:?}",
                    e
                )
            })
    }
}

pub async fn init_serve_connection(
    run_mode: RunMode,
) -> Result<(Connection, String), Box<dyn Error>> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info!("Init Proxy version {VERSION}");

    let proxy = SysDManagerProxy::new()?;

    let id = unsafe { libc::getegid() };
    info!("User id {id}");

    let default_name = if run_mode == RunMode::Development {
        DBUS_NAME_DEV
    } else {
        DBUS_NAME
    };

    let dbus_name = get_env("DBUS_NAME", default_name);
    let dbus_path = get_env("DBUS_PATH", DBUS_PATH);

    info!("DBus name {dbus_name}");
    info!("DBus path {dbus_path}");

    let connection = connection::Builder::system()?
        .name(dbus_name)?
        .serve_at(dbus_path.clone(), proxy)?
        .build()
        .await?;

    Ok((connection, dbus_path.to_string()))
}

fn get_env<'a>(key: &str, default: &'a str) -> Cow<'a, str> {
    match env::var(key) {
        Ok(val) => {
            info!("Key {key}, Value {val}");
            Cow::Owned(val)
        }
        Err(e) => {
            debug!("Env error {e:?}");
            info!("Key {key}, Use default value {default}");
            Cow::Borrowed(default)
        }
    }
}

async fn get_proxy() -> Result<&'static sysdcom::SysDManagerComLinkProxy<'static>, zbus::Error> {
    system_proxy().await //Only system cause the proxy runs at root so no session
}

static SYS_PROXY: OnceCell<sysdcom::SysDManagerComLinkProxy> = OnceCell::const_new();

async fn system_proxy() -> Result<&'static sysdcom::SysDManagerComLinkProxy<'static>, zbus::Error> {
    SYS_PROXY
        .get_or_try_init(
            async || -> Result<sysdcom::SysDManagerComLinkProxy, zbus::Error> {
                let connection = Connection::system().await?;
                let proxy = sysdcom::SysDManagerComLinkProxy::builder(&connection)
                    .build()
                    .await?;
                Ok(proxy)
            },
        )
        .await
}
