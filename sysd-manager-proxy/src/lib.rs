use log::info;
use zbus::interface;

pub struct Greeter {
    pub count: u64,
}

#[interface(name = "org.zbus.MyGreeter1")]
impl Greeter {
    // Can be `async` as well.
    pub async fn say_hello(&mut self, name: &str) -> String {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);
        self.count += 1;
        format!("Hello {}! I have been called {} times.", name, self.count)
    }

    pub async fn create_dropin(&mut self, name: &str) -> String {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);

        format!("Create DropIn {:?}!", name)
    }

    pub async fn my_user_id(&mut self) -> u32 {
        let id = unsafe { libc::getegid() };
        info!("id {}", id);
        id
    }
}
