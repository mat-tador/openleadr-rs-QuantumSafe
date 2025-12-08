use std::net::SocketAddr;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use axum_server::tls_openssl::OpenSSLConfig; // <--- NUOVO IMPORT

#[cfg(feature = "postgres")]
use openleadr_vtn::data_source::PostgresStorage;
use openleadr_vtn::{data_source::Migrate, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();

    // --- CONFIGURAZIONE HTTPS ---
    // Carichiamo i certificati CLASSICI generati prima
    let ssl_config = OpenSSLConfig::from_pem_file(
        "certs/classic/server_cert.pem", // Certificato Pubblico
        "certs/classic/server_key.pem",  // Chiave Privata RSA
    ).expect("Impossibile caricare i certificati. Sei nella cartella giusta?");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("🚀 VTN in ascolto su HTTPS://{}", addr);

    #[cfg(feature = "postgres")]
    let storage = PostgresStorage::from_env().await.unwrap();

    #[cfg(not(feature = "postgres"))]
    compile_error!("No storage backend selected.");

    if let Err(e) = storage.migrate().await {
        warn!("Database migration failed: {}", e);
    }

    let state = AppState::new(storage).await;
    let app = state.into_router();

    // --- AVVIO SERVER HTTPS ---
    // Sostituiamo axum::serve con axum_server
    match axum_server::bind_openssl(addr, ssl_config)
        .serve(app.into_make_service())
        .await 
    {
        Ok(_) => info!("Server chiuso correttamente"),
        Err(e) => error!("Server crashato: {}", e),
    }
}

// Nota: axum-server gestisce lo shutdown diversamente, rimosso per brevità