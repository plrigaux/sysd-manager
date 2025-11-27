use std::sync::OnceLock;

use log::info;
use zbus::{Connection, ObjectServer, interface, message::Header, object_server::SignalEmitter};
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};
static AUTHORITY: OnceLock<AuthorityProxy> = OnceLock::new();

pub async fn init_authority() -> Result<(), zbus::Error> {
    let connection = Connection::system().await?;
    let proxy = AuthorityProxy::new(&connection).await?;

    info!("backend name {}", proxy.backend_name().await?);
    info!("backend version {}", proxy.backend_version().await?);
    info!("backend feature {:?}", proxy.backend_features().await?);

    AUTHORITY.get_or_init(|| proxy);
    Ok(())
}

pub fn auth() -> &'static AuthorityProxy<'static> {
    AUTHORITY.get().expect("REASON")
}

pub struct SysDManagerProxy {
    pub subject: Subject,
    //pub test: usize,
}

impl SysDManagerProxy {
    pub fn new() -> Result<Self, zbus_polkit::Error> {
        let subject = Subject::new_for_owner(std::process::id(), None, None)?;
        Ok(SysDManagerProxy { subject })
    }
}

#[interface(name = "io.github.plrigaux.SysDManager", introspection_docs = true)]
impl SysDManagerProxy {
    /*     pub fn new(auth: AuthorityProxy, subject: Subject) -> Self {
        let t = SysDManagerProxy { test: 2 };
        t
    } */
    // Can be `async` as well.

    pub async fn create_dropin(&mut self, file_name: &str, _content: &str) -> String {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);

        format!("Create DropIn {:?}!", file_name)
    }

    pub async fn save_file(&mut self, file_name: &str, _content: &str) -> String {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);

        format!("Create DropIn {:?}!", file_name)
    }

    pub async fn my_user_id(&mut self) -> u32 {
        let a = auth();
        let r = a
            .check_authorization(
                &self.subject,
                "io.github.plrigaux.SysDManager",
                &std::collections::HashMap::new(),
                CheckAuthorizationFlags::AllowUserInteraction.into(),
                "",
            )
            .await;
        let id = unsafe { libc::getegid() };
        info!("id {}", id);
        id
    }
    // "Bye" signal (note: no implementation body).
    #[zbus(signal)]
    async fn bye(signal_emitter: &SignalEmitter<'_>, message: &str) -> zbus::Result<()>;

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

    async fn even_ping(&mut self, val: u32) -> zbus::fdo::Result<u32> {
        if val.is_multiple_of(2) {
            Ok(val)
        } else {
            Err(zbus::fdo::Error::Failed(format!("{val} not even!")))
        }
    }
}
