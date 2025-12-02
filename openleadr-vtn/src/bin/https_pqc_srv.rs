use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use axum_server::tls_openssl::OpenSSLConfig;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslOptions};
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

    // 2. Crea l'acceptor TLS con metodo base
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    // 3. CARICAMENTO CERTIFICATI ECC (Standard)
    info!("📂 Carico Certificati ECC (Identity)...");
    acceptor.set_private_key_file("certs/classic/server_key.pem", SslFiletype::PEM).unwrap();
    acceptor.set_certificate_chain_file("certs/classic/server_cert.pem").unwrap();
    acceptor.check_private_key().unwrap();

    // 4. LA MAGIA QUANTISTICA: Forziamo Kyber e TLS 1.3
    info!("🛡️  Attivo Key Exchange Post-Quantum e forzo TLS 1.3...");
    
    // A. FORZATURA KYBER (Key Exchange Groups)
    // CORREZIONE: Usa set_groups_list() invece di set_groups()
    // Nota: usa "mlkem768" (FIPS 203 standard) o "kyber768" se hai la versione Round 3
    acceptor.set_groups_list("x25519_mlkem768:mlkem768:x25519")
        .expect("❌ Errore: Il provider non supporta mlkem768/kyber768.");
    
    // B. FORZATURA TLS 1.3 (Protocol Version)
    // Disabilita esplicitamente TLS 1.2 e versioni precedenti
    acceptor.set_options(
        SslOptions::NO_TLSV1_2 | SslOptions::NO_TLSV1_1 | SslOptions::NO_TLSV1
    );
    
    info!("🚀 Forzatura TLS 1.3 (disabilitando TLS 1.2) completata.");

    let ssl_config = OpenSSLConfig::from_acceptor(Arc::new(acceptor.build()));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    
    info!("🚀 VTN HYBRID SERVER attivo su HTTPS://{}", addr);
    info!("   - Protocollo: TLS 1.3");
    info!("   - Key Exchange: X25519_MLKEM768 (Hybrid), MLKEM768, X25519");
    info!("   - Auth: ECC (dal tuo certificato)");

    // 5. AVVIO NORMALE
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