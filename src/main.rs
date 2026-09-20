//! figma-import — one-way `.fig` → normalized JSON conversion service.
//!
//! Not a Figma/design-tool embed and not an editor: this process takes a
//! binary `.fig` upload, runs it through `op-figma`'s real Kiwi-binary
//! parser (vendored from ZSeven-W/openpencil, MIT), and returns the
//! resulting `PenDocument` tree plus a flattened summary (text content,
//! image references, per-type node counts). See `src/fig.rs` for exactly
//! which Figma features do not round-trip.
//!
//! The router itself lives in `lib.rs` so the integration test can drive
//! it in-process via `tower::ServiceExt::oneshot`; this binary just binds
//! it to a real socket.

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "figma_import=info,tower_http=info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let app = figma_import::build_router();

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));
    tracing::info!("figma-import listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
