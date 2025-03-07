use axum::{routing::get, Router};
use clap::Parser;

use log::{info, warn};
use tokio::signal::unix::{signal, SignalKind};

use std::io::Write;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
    #[arg(short, long, default_value = "127.0.0.1")]
    pub addr: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        //.format_target(false)
        //.format_timestamp(None)
        .format(|buf, record| {
            let style = buf
                .default_level_style(record.level())
                .effects(anstyle::Effects::BOLD);
            writeln!(buf, "{style}{}{style:#} {}", record.level(), record.args())
        })
        .filter_level(log::LevelFilter::Info)
        .init();

    let ret: Result<(), std::io::Error> = setup_server().await;

    if let Err(e) = ret {
        warn!("Error: {:?}", e);
        return Err(e);
    }

    Ok(())
}

async fn setup_server() -> std::io::Result<()> {
    let args = Args::parse();
    let port = args.port;
    let ip_addr = args.addr;

    let mut signal_terminate = signal(SignalKind::terminate())?;
    let mut signal_interrupt = signal(SignalKind::interrupt())?;
    let mut signal_quit = signal(SignalKind::quit())?;

    let mut signal_alarm = signal(SignalKind::alarm())?;
    let mut signal_hanghup = signal(SignalKind::hangup())?;
    let sig_min_1 = libc::SIGRTMIN() + 1;
    let mut signal_rt1 = signal(SignalKind::from_raw(sig_min_1))?;
    info!("SIGRTMIN() + 1 = {sig_min_1}");

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = signal_terminate.recv() => terminate("SIGTERM."),
                _ = signal_interrupt.recv() => terminate("SIGINT."),
                _ = signal_quit.recv() => terminate("SIGQUIT."),
                _ = signal_alarm.recv() => info!("Received SIGALRM."),
                _ = signal_hanghup.recv() => info!("Received SIGHUP."),
                _ = signal_rt1.recv() => info!("Received SIGRTMIN + 1."),
            }
        }
    });

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/hey", get(manual_hello));

    let addr = format!("{ip_addr}:{port}");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr.clone()).await?;
    let local_addr = listener.local_addr()?;
    info!("Tiny Daemon listening on {:?}", local_addr);
    axum::serve(listener, app)
        //.with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn terminate(signal: &str) {
    info!("Received {signal}.");
    warn!("Exiting");
    std::process::exit(0);
}

async fn root() -> &'static str {
    info!("root --> \"Hello, World!\"");
    "Hello, World!"
}

async fn manual_hello() -> &'static str {
    info!("manual_hello --> \"Hey there!\"");
    "Hey there!"
}

/* async fn shutdown_signal() {

    loop {
        tokio::select! {
            _ =  async {
                signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {
                info!("interrupt"); break;},
            _ = async {
                signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {info!("terminate");
            break;},
            _ =  async {
                signal::unix::signal(signal::unix::SignalKind::hangup())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {info!("hangup")},
        }
    }

    info!("Exiting");
    std::process::exit(0);
}
 */
