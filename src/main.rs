#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::Router;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use std::env;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    webcaldav::init_executor();

    let port = env::var("PORT").unwrap_or_else(|_| "8090".to_string());
    for (key, default) in [
        ("LEPTOS_OUTPUT_NAME", "webcaldav"),
        ("LEPTOS_SITE_ROOT", "target/site"),
        ("LEPTOS_SITE_PKG_DIR", "pkg"),
        ("LEPTOS_SITE_ADDR", &format!("127.0.0.1:{port}")),
        ("LEPTOS_RELOAD_PORT", "3012"),
    ] {
        if env::var(key).is_err() {
            env::set_var(key, default);
        }
    }
    let leptos_options = leptos::config::get_configuration(None)
        .expect("failed to load leptos config")
        .leptos_options;

    let routes = generate_route_list(webcaldav::App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let options = leptos_options.clone();
            move || webcaldav::shell(options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(webcaldav::shell))
        .with_state(leptos_options.clone());

    let addr: std::net::SocketAddr = format!("0.0.0.0:{port}").parse()?;
    tracing::info!("webcaldav listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // No client-side main function needed; see lib.rs's hydrate() instead.
}
