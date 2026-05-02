use std::{error::Error, net::SocketAddr, path::PathBuf};

use axum::{Json, Router, extract::State, routing::{delete, get, post}};
use axum_embed::ServeEmbed;
use serde::Serialize;
use serde_json::json;
use sysinfo::{System, get_current_pid};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use rust_embed::RustEmbed;

use crate::{config::Config, create_client};

#[derive(RustEmbed, Clone)]
#[folder = "./web/DropSentry/dist"]
#[include = "*"]
struct FrontendAssets;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub home_dir: PathBuf,
}

pub async fn start_api (state: AppState) -> Result<(), Box<dyn Error>> {
    let api_routes = Router::new()
        .route("/performance", get(performance))
        .route("/games", get(get_games))
        .route("/games", post(add_game))
        .route("/games", delete(delete_game))
        .route("/games/reorder", post(reorder_game))
        .route("/proxies", get(get_proxies))
        .route("/proxies", post(add_proxy))
        .route("/proxies", delete(delete_proxy))
        .route("/create_client", post(create_new_client));   

    let frontend_service = ServeEmbed::<FrontendAssets>::new();

    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(frontend_service)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("http://{}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}


//performance
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

//game
#[derive(Serialize)]
pub struct Game {
    pub name: String,
    pub position: usize,
}

async fn get_games(State(state): State<AppState>) -> Result<Json<Vec<Game>>, axum::http::StatusCode> {
    let games = state.config.loaded_games().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let games: Vec<Game> = games.into_iter().enumerate().map(|(index, name)| Game {
        name,
        position: index
    }).collect();
    Ok(Json(games))
}

async fn add_game(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Json<Game> {
    let name = payload["name"].as_str().unwrap_or_default().to_string();
    let position = state.config.add_game(&name).await.unwrap();
    Json(Game { name, position })
}

async fn reorder_game(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let game_name = payload["game_name"].as_str().unwrap_or_default().to_string();
    let new_position = payload["position"].as_u64().unwrap_or(1) as usize;

    state.config.reorder_game(&game_name, new_position).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn delete_game(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let pos = payload["position"].as_u64().unwrap_or(1) as usize;
    state.config.delete_game(pos).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

//proxies
async fn get_proxies(State(state): State<AppState>) -> Result<Json<Vec<String>>, axum::http::StatusCode> {
    let proxies = state.config.load_proxies_list().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(proxies))
}

async fn add_proxy(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let url = payload["url"].as_str().unwrap_or_default().to_string();
    state.config.add_proxy(&url).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "url": url })))
}

async fn delete_proxy(State(state): State<AppState>, Json(payload): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let proxy = payload["url"].as_str().unwrap_or_default().to_string();
    state.config.delete_proxy(&proxy).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

//dashbroad
async fn create_new_client(State(state): State<AppState>) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let proxies = state.config.load_proxies_list().await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    create_client(&state.home_dir, &proxies).await.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}