use std::{error::Error, net::{SocketAddr}};

use axum::{Json, Router, routing::get};
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

pub async fn start_api () -> Result<(), Box<dyn Error + Send + Sync>> {
    let api_routes = Router::new().route("/hello", get(hello));
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new("../frontend/dist"))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Сервер запущен на http://{}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn hello() -> Json<serde_json::Value> {
    Json(json!({ "message": "hello" }))
}