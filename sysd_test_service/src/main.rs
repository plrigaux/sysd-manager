use axum::{
    routing::get, Router,
};
use clap::Parser;

use log::info;
use signal_hook::{
    consts::{SIGHUP, SIGINT, SIGTERM},
    iterator::Signals,
};


use tokio::signal;


#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let port = args.port;

    let mut signals = Signals::new(&[SIGTERM, SIGHUP, SIGINT])?;

    tokio::spawn({
        async move {
            for sig in signals.forever() {
                info!("Received signal {:?}", sig);

                if sig == SIGINT {
                    info!("cancel_token.cancel()");
                }
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
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn manual_hello() -> &'static str {
    "Hey there!"
}

async fn shutdown_signal() {

    loop {
        tokio::select! {
            _ =  async {
                signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {
                println!("interrupt"); break;},
            _ = async {
                signal::unix::signal(signal::unix::SignalKind::interrupt())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {println!("terminate");
            break;},
            _ =  async {
                signal::unix::signal(signal::unix::SignalKind::hangup())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            } => {println!("hangup")},
        }
    }

    println!("Exiting");
    std::process::exit(0);
}
