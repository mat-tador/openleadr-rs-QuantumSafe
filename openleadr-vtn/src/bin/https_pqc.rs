use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslVersion, SslVerifyMode};
use openssl::provider::Provider;
use axum_server::tls_openssl::OpenSSLConfig;

#[cfg(feature = "postgres")]
use openleadr_vtn::data_source::PostgresStorage;
use openleadr_vtn::{data_source::Migrate, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();

    // 1. CARICAMENTO PROVIDER
    // Carichiamo OQS. Se fallisce, il log ci avvisa ma non crasha tutto subito.
    if let Err(e) = Provider::try_load(None, "oqsprovider", true) {
        warn!("⚠️ OQS Provider non caricato: {}. Kyber non funzionerà.", e);
    } else {
        info!("✅ OQS Provider caricato!");
    }
    // Carichiamo il default per le operazioni base
    let _default = Provider::try_load(None, "default", true);

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls_server())
        .expect("Impossibile creare builder");

    // 2. SETUP SICUREZZA
    builder.set_security_level(0);
    builder.set_verify(SslVerifyMode::NONE);
    
    // TLS 1.3 Obbligatorio (ECDSA + Kyber lavorano bene qui)
    builder.set_min_proto_version(Some(SslVersion::TLS1_3)).unwrap();

    // 3. GRUPPI (Key Exchange)
    // Qui definiamo COSA usiamo per scambiare le chiavi.
    // - x25519_kyber768: Il nostro obiettivo PQC
    // - prime256v1: NECESSARIO per leggere il certificato ECDSA!
    // - x25519: Fallback
    let groups = "x25519_kyber768:kyber768:prime256v1:x25519";
    builder.set_groups_list(groups).expect("Errore setup gruppi");
    info!("✅ Gruppi impostati: {}", groups);

    // 4. RIMOSSO set_sigalgs_list (IMPORTANTE!)
    // Lasciamo che OpenSSL scelga automaticamente ECDSA

    // 5. CARICAMENTO CERTIFICATI (ECDSA)
    let cert_path = "/app/certs/pqc/ec_cert.pem";
    let key_path = "/app/certs/pqc/ec_key.pem";
    
    info!("Caricamento certificati ECDSA da: {}", cert_path);
    builder.set_private_key_file(key_path, SslFiletype::PEM).expect("Chiave non trovata");
    builder.set_certificate_chain_file(cert_path).expect("Certificato non trovato");

    // Avvio Server
    let acceptor = builder.build();
    let ssl_config = OpenSSLConfig::from_acceptor(Arc::new(acceptor));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    
    info!("🚀 VTN in ascolto su HTTPS://{}", addr);

    #[cfg(feature = "postgres")]
    let storage = PostgresStorage::from_env().await.unwrap();
    #[cfg(not(feature = "postgres"))]
    compile_error!("No storage backend selected.");

    if let Err(e) = storage.migrate().await { warn!("DB: {}", e); }
    let state = AppState::new(storage).await;
    let app = state.into_router();

    axum_server::bind_openssl(addr, ssl_config)
        .serve(app.into_make_service())
        .await.unwrap();
}