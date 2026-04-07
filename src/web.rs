use std::{error::Error, net::{SocketAddr}};

use axum::{Json, Router, routing::get};
use axum_embed::ServeEmbed;
use serde::Serialize;
use serde_json::json;
use sysinfo::{System, get_current_pid};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "./web/DropSentry/dist"]
#[include = "*"]
struct FrontendAssets;

pub async fn start_api () -> Result<(), Box<dyn Error + Send + Sync>> {
    let api_routes = Router::new()
        .route("/performance", get(performance));

    let frontend_service = ServeEmbed::<FrontendAssets>::new();

    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(frontend_service)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("http://{}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Serialize)]
struct SystemInfo {
    total_memory: u64,
    process_memory: u64,
    cpu_name: String,
    cpu_usage: f32,
}

async fn performance () -> Json<SystemInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_name = sys.cpus().first().map(|s| s.brand()).unwrap_or_default().to_string();
    let (process_memory, cpu_usage) = if let Ok(pid) = get_current_pid() {
        if let Some(process) = sys.process(pid) {
            (process.memory(), process.cpu_usage())
        } else {
            (0, 0.0)
        }
    } else {
        (0, 0.0)
    };
    
    Json(SystemInfo { total_memory: sys.total_memory(), process_memory, cpu_name, cpu_usage })
}