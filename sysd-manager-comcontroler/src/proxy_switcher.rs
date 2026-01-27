use std::sync::{LazyLock, RwLock};

pub const KEY_PREF_USE_PROXY_CLEAN: &str = "pref-use-proxy-clean";
pub const KEY_PREF_USE_PROXY_FREEZE: &str = "pref-use-proxy-freeze";
pub const KEY_PREF_USE_PROXY_THAW: &str = "pref-use-proxy-thaw";
pub const KEY_PREF_USE_PROXY_ENABLE_UNIT_FILE: &str = "pref-use-proxy-enable-unit-file";
pub const KEY_PREF_USE_PROXY_DISABLE_UNIT_FILE: &str = "pref-use-proxy-disable-unit-file";
pub const KEY_PREF_USE_PROXY_RELOAD_DAEMON: &str = "pref-use-proxy-reload-daemon";
pub const KEY_PREF_USE_PROXY_CREATE_DROP_IN: &str = "pref-use-proxy-create-drop-in";
pub const KEY_PREF_USE_PROXY_SAVE_FILE: &str = "pref-use-proxy-save-file";
pub const KEY_PREF_USE_PROXY_REVERT_UNIT_FILE: &str = "pref-use-proxy-revert-unit-file";
pub const KEY_PREF_PROXY_START_AT_STARTUP: &str = "pref-proxy-start-at-startup";
pub const KEY_PREF_PROXY_STOP_AT_CLOSE: &str = "pref-proxy-stop-at-close";

pub static PROXY_SWITCHER: LazyLock<ProxySwitcher> = LazyLock::new(|| {
    let ps = ProxySwitcher::default();
    #[cfg(not(feature = "flatpak"))]
    {
        use base::consts::APP_ID;
        use gio::prelude::SettingsExt;

        let settings = gio::Settings::new(APP_ID);
        let val = settings.boolean(KEY_PREF_USE_PROXY_CLEAN);
        ps.set_clean(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_FREEZE);
        ps.set_freeze(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_THAW);
        ps.set_thaw(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_ENABLE_UNIT_FILE);
        ps.set_enable_unit_file(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_DISABLE_UNIT_FILE);
        ps.set_disable_unit_file(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_RELOAD_DAEMON);
        ps.set_reload(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_CREATE_DROP_IN);
        ps.set_create_dropin(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_SAVE_FILE);
        ps.set_save_file(val);
        let val = settings.boolean(KEY_PREF_USE_PROXY_REVERT_UNIT_FILE);
        ps.set_revert_unit_file(val);
        let val = settings.boolean(KEY_PREF_PROXY_START_AT_STARTUP);
        ps.set_start_at_startup(val);
        let val = settings.boolean(KEY_PREF_PROXY_STOP_AT_CLOSE);
        ps.set_stop_at_close(val);
    }
    ps
});

#[derive(Default)]
pub struct ProxySwitcher {
    clean: RwLock<bool>,
    freeze: RwLock<bool>,
    thaw: RwLock<bool>,
    enable_unit_file: RwLock<bool>,
    disable_unit_file: RwLock<bool>,
    reload: RwLock<bool>,
    create_dropin: RwLock<bool>,
    save_file: RwLock<bool>,
    revert_unit_file: RwLock<bool>,
    start_at_start_up: RwLock<bool>,
    stop_at_close: RwLock<bool>,
}

impl ProxySwitcher {
    pub fn clean(&self) -> bool {
        *self.clean.read().unwrap()
    }

    pub fn set_clean(&self, value: bool) {
        *self.clean.write().unwrap() = value;
    }

    pub fn freeze(&self) -> bool {
        *self.freeze.read().unwrap()
    }

    pub fn set_freeze(&self, value: bool) {
        *self.freeze.write().unwrap() = value;
    }

    pub fn thaw(&self) -> bool {
        *self.thaw.read().unwrap()
    }

    pub fn set_thaw(&self, value: bool) {
        *self.thaw.write().unwrap() = value;
    }

    pub fn enable_unit_file(&self) -> bool {
        *self.enable_unit_file.read().unwrap()
    }

    pub fn set_enable_unit_file(&self, value: bool) {
        *self.enable_unit_file.write().unwrap() = value;
    }

    pub fn disable_unit_file(&self) -> bool {
        *self.disable_unit_file.read().unwrap()
    }

    pub fn set_disable_unit_file(&self, value: bool) {
        *self.disable_unit_file.write().unwrap() = value;
    }

    pub fn save_file(&self) -> bool {
        *self.save_file.read().unwrap()
    }

    pub fn set_save_file(&self, value: bool) {
        *self.save_file.write().unwrap() = value;
    }

    pub fn create_dropin(&self) -> bool {
        *self.create_dropin.read().unwrap()
    }

    pub fn set_create_dropin(&self, value: bool) {
        *self.create_dropin.write().unwrap() = value;
    }

    pub fn start_at_start_up(&self) -> bool {
        *self.start_at_start_up.read().unwrap()
    }

    pub fn set_start_at_startup(&self, value: bool) {
        *self.start_at_start_up.write().unwrap() = value;
    }

    pub fn stop_at_close(&self) -> bool {
        *self.stop_at_close.read().unwrap()
    }

    pub fn set_stop_at_close(&self, value: bool) {
        *self.stop_at_close.write().unwrap() = value;
    }

    pub fn revert_unit_file(&self) -> bool {
        *self.revert_unit_file.read().unwrap()
    }

    pub fn set_revert_unit_file(&self, value: bool) {
        *self.revert_unit_file.write().unwrap() = value;
    }

    pub fn reload(&self) -> bool {
        *self.reload.read().unwrap()
    }

    pub fn set_reload(&self, value: bool) {
        *self.reload.write().unwrap() = value;
    }

    pub fn uses_any_proxy(&self) -> bool {
        self.clean()
            || self.freeze()
            || self.thaw()
            || self.create_dropin()
            || self.disable_unit_file()
            || self.reload()
            || self.save_file()
            || self.enable_unit_file()
            || self.revert_unit_file()
    }
}
