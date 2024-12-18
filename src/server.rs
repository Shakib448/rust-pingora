use axum::Router;
use axum::routing::get;
use std::env;
use axum::http::HeaderMap;



async fn health(headers: HeaderMap) -> String {
    let proxy = headers.get("x-proxy-from").and_then(|value| value.to_str().ok()).unwrap_or("unknown");
    format!("Up, answered from 0.0.0.0:{} with {}", env::var("PORT").unwrap_or_else(|_| "".to_string()), proxy)
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new()
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();

    println!("Server running at {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .unwrap();
}