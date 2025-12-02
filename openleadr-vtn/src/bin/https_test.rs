use std::net::SocketAddr;
use std::sync::Arc;
use std::fs; 
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// --- IMPORT ESSENZIALI DEL PROGETTO ---
use openleadr_vtn::{data_source::Migrate, state::AppState}; // FIX: AppState/Migrate importati correttamente
#[cfg(feature = "postgres")]
use openleadr_vtn::data_source::PostgresStorage; // FIX: PostgresStorage importato correttamente

// --- IMPORT CRITTOGRAFIA/HTTP ---
use openssl::pkey::{PKey, Private}; 
use openssl::sign::Signer;
use openssl::hash::MessageDigest;

use axum_server::tls_openssl::OpenSSLConfig; 
use axum::{
    body::Body,
    extract::Extension,
    middleware::Next, 
    response::Response,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm}; 
use serde::Deserialize; // Mantenuto solo Deserialize per il debug, ma non essenziale qui
use serde_json::Value as JsonValue; // Usiamo Value per il payload generico

// Alias per la chiave privata del server (VTN)
type VtnSigningKey = Arc<PKey<Private>>; 
// FIX 1: Definiamo EncodingKey come Arc<EncodingKey>
type VtnEncodingKey = Arc<EncodingKey>; 
// FIX 2: Definiamo il payload come tipo generico JSON
type JwsPayload = JsonValue; 


// Funzione helper per firmare un payload
fn sign_data(key: &PKey<Private>, data: &[u8]) -> Result<Vec<u8>, openssl::error::ErrorStack> {
    let mut signer = Signer::new(MessageDigest::sha256(), key)?;
    signer.update(data)?;
    signer.sign_to_vec()
}


// --- MIDDLEWARE DI FIRMA (JWS Signer) ---
async fn add_signature_middleware(
    // FIX 3: Riferimento al tipo corretto (Arc<EncodingKey>)
    Extension(encoding_key): Extension<VtnEncodingKey>, 
    req: Request<Body>, 
    next: Next 
) -> Response {
    let response = next.run(req).await;

    if response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED {
        let (mut parts, body) = response.into_parts();
        
        let bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return Response::from_parts(parts, Body::empty()),
        };
        
        // 1. Deserializza il corpo in un payload JSON generico
        let payload: JwsPayload = serde_json::from_slice(&bytes).unwrap_or_default();
        
        // 2. FIRMA JWS
        // FIX 4: Uso dell'algoritmo RS256 (corretta capitalizzazione)
        let token = match encode(&Header::new(Algorithm::RS256), &payload, encoding_key.as_ref()) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Firma JWS fallita: {}", e);
                return Response::from_parts(parts, Body::from(bytes));
            }
        };

        // 3. Ricreiamo la Risposta e aggiungiamo l'Header
        let new_body = Body::from(bytes);
        let mut final_response = Response::from_parts(parts, new_body);
        
        final_response.headers_mut().insert(
            "X-Server-Signature-JWS", 
            token.parse().unwrap(),
        );

        return final_response;
    }

    response
}


#[tokio::main]
async fn main() {
    // ... (Log initialization) ...
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();

    // --- CARICAMENTO CHIAVE DI FIRMA (RSA CLASSICA) ---
    let server_key_pem = fs::read("certs/classic/server_key.pem")
        .expect("Manca certs/classic/server_key.pem per la firma!");
    
    let private_key = PKey::private_key_from_pem(&server_key_pem)
        .expect("Errore nel parsing della chiave privata RSA");
        
    // 1. Creiamo la chiave JWT EncodingKey
    // FIX 5: Creiamo la EncodingKey dal pem e la condividiamo
    let encoding_key: VtnEncodingKey = Arc::new(EncodingKey::from_rsa_pem(&server_key_pem).unwrap());
    
    // --- CONFIGURAZIONE HTTPS ---
    let ssl_config = OpenSSLConfig::from_pem_file(
        "certs/classic/server_cert.pem", 
        "certs/classic/server_key.pem", 
    ).expect("Impossibile caricare i certificati TLS.");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("🚀 VTN in ascolto su HTTPS://{}", addr);

    // FIX 6: Inizializzazione storage e stato (la logica che mancava)
    #[cfg(feature = "postgres")]
    let storage = PostgresStorage::from_env().await.unwrap();

    #[cfg(not(feature = "postgres"))]
    compile_error!("No storage backend selected.");

    if let Err(e) = storage.migrate().await {
        warn!("Database migration failed: {}", e);
    }

    let state = AppState::new(storage).await;
    let app_router = state.into_router();

    // --- INIEZIONE MIDDLEWARE DI FIRMA ---
    let app = app_router
        // 1. Iniettiamo la EncodingKey (Arc<EncodingKey>)
        .layer(Extension(encoding_key)) 
        // 2. Aggiungiamo il middleware che CONSUMA l'estensione
        .layer(axum::middleware::from_fn(add_signature_middleware));
    
    // --- AVVIO SERVER HTTPS ---
    match axum_server::bind_openssl(addr, ssl_config)
        .serve(app.into_make_service())
        .await 
    {
        Ok(_) => info!("Server chiuso correttamente"),
        Err(e) => error!("Server crashato: {}", e),
    }
}