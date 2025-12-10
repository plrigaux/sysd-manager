use log::error;
use sm_proxy::{install::install, serve_proxy};
use systemd::runtime;

pub(super) fn install_flatpak_proxy() {
    runtime().block_on(async {
        if let Err(err) = install(base::RunMode::Normal).await {
            error!("Installation error: {err:?}")
        }
    });
}

pub(super) fn run_flatpak_proxy() {
    runtime().block_on(async {
        if let Err(err) = serve_proxy(base::RunMode::Normal).await {
            error!("Installation error: {err:?}")
        }
    });
}
