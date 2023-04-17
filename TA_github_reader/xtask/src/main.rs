#[cfg(feature = "deploy")]
use axum::Router;
use std::{env, fs};
#[cfg(feature = "deploy")]
use std::net::SocketAddr;
#[cfg(feature = "deploy")]
use tower_http::{services::ServeDir, trace::TraceLayer};
#[cfg(feature = "deploy")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use xshell::{cmd, Shell};

type DynError = Box<dyn std::error::Error + Sync + Send + 'static>;

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

async fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("build") => build()?,
        #[cfg(feature = "deploy")]
        Some("deploy") => deploy().await?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    #[cfg(not(feature = "deploy"))]
    println!("bulid");
    #[cfg(feature = "deploy")]
    println!("bulid, deploy");
}

// TODO
// Accept target app as param
fn build() -> Result<(), DynError> {
    let sh = Shell::new()?;
    cmd!(
        sh,
        "cargo build -Z build-std=std --bin github_reader --target x86_64-unknown-linux-musl --release"
    )
        .run()?;
    fs::create_dir_all("TA_github_reader/linux_x86_64/bin")?;
    cmd!(
        sh,
        "cp target/x86_64-unknown-linux-musl/release/github_reader TA_github_reader/linux_x86_64/bin/github"
    )
    .run()?;
    cmd!(sh, "tar cvf github_reader.tar TA_github_reader").run()?;
    cmd!(sh, "gzip -f -9 github_reader.tar ").run()?;
    Ok(())
}

// TODO
// Accept Splunk host as param
// Accept Source location as param
// Accept app as param
#[cfg(feature = "deploy")]
async fn deploy() -> Result<(), DynError> {
    build()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_static_file_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tokio::spawn(async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], 3005));
        tracing::debug!("listening on {}", addr);
        let app = Router::new().nest_service("/", ServeDir::new("."));
        axum::Server::bind(&addr)
            .serve(app.layer(TraceLayer::new_for_http()).into_make_service())
            .await
            .unwrap();
    });

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;
    let resp = client
        .post("https://localhost:8089/services/apps/local")
        .basic_auth("admin", Some("aaaaaaaa"))
        .body("name=http://192.168.3.66:3005/github_reader.tar.gz&filename=true&update=true&visible=true")
        .send()
        .await?;
    let status = resp.status();
    println!("{}:{:?}", &resp.text().await?, status);

    Ok(())
}
