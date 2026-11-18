#![warn(clippy::all)]
// rsx! erzeugt format!-Aufrufe für Attribute
#![allow(clippy::useless_format)]
#![deny(clippy::unwrap_used)]

mod api;
mod app;
mod components;
mod domain;
mod model;
mod util;

#[cfg(feature = "server")]
mod server;

use app::App;

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(App);

    #[cfg(feature = "server")]
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        // Docker-Healthcheck: nimmt der Server Verbindungen an?
        let port = std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(8080);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let ok = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3)).is_ok();
        std::process::exit(i32::from(!ok));
    }

    #[cfg(feature = "server")]
    dioxus::serve(|| async {
        dotenv::dotenv().ok();
        server::set_pool(server::init_db().await);
        Ok(dioxus::server::router(App).nest("/api/v1", server::rest::router()))
    });
}
