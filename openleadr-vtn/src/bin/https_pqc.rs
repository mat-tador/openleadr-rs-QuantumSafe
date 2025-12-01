use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use axum_server::tls_openssl::OpenSSLConfig;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use openssl::provider::Provider;

#[cfg(feature = "postgres")]
use openleadr_vtn::data_source::PostgresStorage;
use openleadr_vtn::{data_source::Migrate, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // 1. CARICAMENTO PROVIDER (Fondamentale per Kyber)
    openssl::init();
    let _pqc = Provider::try_load(None, "oqsprovider", true)
        .expect("❌ ERRORE: oqsprovider non trovato. Hai settato OPENSSL_MODULES?");
    let _def = Provider::try_load(None, "default", true).unwrap();
    info!("✅ OQS Provider Attivo.");

    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    // 2. CARICAMENTO CERTIFICATI ECC (Standard)
    // Usiamo quelli dalla cartella 'classic' che sappiamo funzionare per l'Handshake.
    info!("📂 Carico Certificati ECC (Identity)...");
    acceptor.set_private_key_file("certs/classic/server_key.pem", SslFiletype::PEM).unwrap();
    acceptor.set_certificate_chain_file("certs/classic/server_cert.pem").unwrap();
    acceptor.check_private_key().unwrap();

    // 3. LA MAGIA QUANTISTICA: Forziamo Kyber
    // CORREZIONE: Usiamo "kyber768" (nome compatibile liboqs 0.10.1)
    info!("🛡️  Attivo Key Exchange Post-Quantum...");
    acceptor.set_groups_list("kyber768:x25519_kyber768:X25519")
        .expect("❌ Errore: Il provider non supporta kyber768.");

    let ssl_config = OpenSSLConfig::from_acceptor(Arc::new(acceptor.build()));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    
    info!("🚀 VTN HYBRID SERVER attivo su HTTPS://{} (Auth: ECC, KeyEx: Kyber)", addr);

    // 4. AVVIO NORMALE
    #[cfg(feature = "postgres")]
    let storage = PostgresStorage::from_env().await.unwrap();
    let state = AppState::new(storage).await;
    let app = state.into_router();

    if let Err(e) = axum_server::bind_openssl(addr, ssl_config)
        .serve(app.into_make_service())
        .await 
    {
        error!("🔥 Server crashato: {}", e);
    }
}