use axum::{routing::{get, post}, Router};
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use rustls::crypto::ring;
use std::thread;
use tracing::{info, error};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeDir};

mod network;
mod database;
mod game;

#[derive(Clone)]
pub struct AppState {
    pub db_tx: std::sync::mpsc::Sender<crate::database::db_thread::DbCommand>,
}

#[tokio::main]
async fn main() {


    let general_appender = tracing_appender::rolling::daily("logs", "growserver.log");
    let (general_writer, _general_guard) = tracing_appender::non_blocking(general_appender);


    let usage_appender = tracing_appender::rolling::daily("logs", "usage.log");
    let (usage_writer, _usage_guard) = tracing_appender::non_blocking(usage_appender);

    use tracing_subscriber::{fmt, prelude::*, EnvFilter};



    let general_layer = fmt::layer()
        .with_writer(general_writer)
        .with_ansi(false)
        .with_target(true)
        .with_filter(EnvFilter::new("info").add_directive("usage=off".parse().unwrap()));



    let usage_layer = fmt::layer()
        .with_writer(usage_writer)
        .with_ansi(false)
        .with_target(false)
        .with_filter(EnvFilter::new("usage=info"));


    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .with_filter(EnvFilter::new("info").add_directive("usage=off".parse().unwrap()));


    tracing_subscriber::registry()
        .with(general_layer)
        .with(usage_layer)
        .with(console_layer)
        .init();


    std::panic::set_hook(Box::new(|info| {
        error!("PANIC: {}", info);
        std::thread::sleep(std::time::Duration::from_secs(10));
    }));

    info!("App started");


    if dotenvy::dotenv().is_err() {
        info!(".env file not found (continuing anyway)");
    }


    info!("Initializing database...");
    if let Err(e) = database::player::init_db() {
        error!("Failed to initialize player database: {}", e);
        std::thread::sleep(std::time::Duration::from_secs(10));
        return;
    }
    if let Err(e) = database::world::init_db() {
        error!("Failed to initialize world database: {}", e);
        std::thread::sleep(std::time::Duration::from_secs(10));
        return;
    }


    info!("Initializing crypto provider...");
    if let Err(_e) = ring::default_provider().install_default() {

    }

    info!("Starting Growtopia Server");


    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<crate::network::server::ServerCommand>();


    info!("Starting Database Thread...");
    let (db_tx, db_rx) = std::sync::mpsc::channel::<crate::database::db_thread::DbCommand>();
    thread::spawn(move || {
        crate::database::db_thread::start_db_thread(db_rx);
    });


    let db_tx_clone = db_tx.clone();
    thread::spawn(move || {
        if let Err(e) = network::server::start_enet_server(cmd_rx, db_tx_clone) {
            eprintln!("ENet server crashed: {}", e);
        }
    });


    thread::spawn(move || {
        use std::io::{self, Write};
        let stdin = io::stdin();
        loop {
            print!("> ");
            io::stdout().flush().ok();
            let mut input = String::new();
            if stdin.read_line(&mut input).is_ok() {
                let parts: Vec<&str> = input.trim().split_whitespace().collect();
                if parts.is_empty() { continue; }

                match parts[0].to_lowercase().as_str() {
                    "give" if parts.len() >= 4 => {
                        let player_name = parts[1].to_string();
                        let item_id: i32 = parts[2].parse().unwrap_or(0);
                        let amount: i32 = parts[3].parse().unwrap_or(0);
                        println!("Sent give command to {}", player_name);
                        cmd_tx.send(crate::network::server::ServerCommand::GiveItem { player_name, item_id, amount }).ok();
                    }
                    "level" if parts.len() >= 3 => {
                        let player_name = parts[1].to_string();
                        let level: i32 = parts[2].parse().unwrap_or(1);
                        println!("Sent level command to {}", player_name);
                        cmd_tx.send(crate::network::server::ServerCommand::SetLevel { player_name, level }).ok();
                    }
                    "xp" if parts.len() >= 3 => {
                        let player_name = parts[1].to_string();
                        let xp: i32 = parts[2].parse().unwrap_or(0);
                        println!("Sent xp command to {}", player_name);
                        cmd_tx.send(crate::network::server::ServerCommand::AddXP { player_name, xp }).ok();
                    }
                    "spawnboss" if parts.len() >= 3 => {
                        let world_name = parts[1].to_string();
                        let health: i32 = parts[2].parse().unwrap_or(100);
                        println!("Sent spawnboss command for {} with hp {}", world_name, health);
                        cmd_tx.send(crate::network::server::ServerCommand::SpawnBoss { world_name, health }).ok();
                    }
                    "help" => {
                        println!("Dev Console Commands:");
                        println!("  give <name> <id> <amount> - Give item to player");
                        println!("  level <name> <level>      - Set player level");
                        println!("  xp <name> <amount>        - Give XP to player");
                        println!("  spawnboss <world> <hp>    - Spawn NPC Boss");
                        println!("  help                      - Show this help");
                    }
                    _ => {
                         println!("Unknown command or wrong arguments. Type 'help' for info.");
                    }
                }
            }
        }
    });


    let state = AppState { db_tx: db_tx.clone() };

    let app = Router::new()
        .route(
            "/growtopia/server_data.php",
            post(crate::network::server_data::server_data).get(crate::network::server_data::server_data),
        )
        .route(
            "/player/login/dashboard",
            axum::routing::any(crate::network::login::dashboard),
        )
        .route(
            "/player/login/discord",
            axum::routing::any(crate::network::login::login_discord),
        )
        .route(
            "/player/growid/login/validate",
            axum::routing::any(crate::network::login::validate),
        )
        .route(
            "/player/growid/checkToken",
            axum::routing::any(crate::network::login::dashboard),
        )
        .route(
            "/discord/callback",
            get(crate::network::discord::handle_discord_callback),
        )
        .nest_service("/cache", ServeDir::new("growtopia_cache/cache"))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .with_state(state);


    let tls = match RustlsConfig::from_pem_file("cert.pem", "key_pkcs8.pem").await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load TLS keys: {}", e);
            std::thread::sleep(std::time::Duration::from_secs(10));
            return;
        }
    };


    let addr = SocketAddr::from(([0, 0, 0, 0], 443));
    info!("Binding HTTPS on 0.0.0.0:443");

    if let Err(e) = axum_server::bind_rustls(addr, tls)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        error!("HTTPS server error: {}", e);
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}