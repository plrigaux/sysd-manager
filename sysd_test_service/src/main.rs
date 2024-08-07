use axum::{
    routing::get, Router,
};
use clap::Parser;

use log::{info, warn};
use signal_hook::{
    consts::{SIGALRM, SIGHUP, SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};


//use tokio::signal;
use dotenv::dotenv;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    let port = args.port;


        let mut signals = Signals::new(&[SIGTERM, SIGQUIT, SIGHUP, SIGINT, SIGALRM])?;

    tokio::spawn({
        async move {
            for sig in signals.forever() {
                info!("Received signal {:?}", sig);

                match sig {
                    SIGTERM | SIGQUIT |SIGINT => {
                        info!("Exiting");
                        std::process::exit(0);
                    },
                    SIGALRM => {
                        warn!("Alarm");
                    },
                    SIGHUP => {
                        info!("signal hang up");
                    },
                    _ => {
                        warn!("Signal not handled");
                    },
                };
            }
        }
    });

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/hey", get(manual_hello));

    let addr = format!("127.0.0.1:{port}");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        //.with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
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