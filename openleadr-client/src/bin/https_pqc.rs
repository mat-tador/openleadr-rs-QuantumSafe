use openleadr_client::{Client, ClientCredentials, Filter, PaginationOptions};
use openleadr_wire::ven::VenContent;
use std::time::Duration;
use tokio::time::sleep;
use uuid::{Uuid, self};
use tracing::{info, error};
use url::{ParseError, Url};
use std::error::Error;
use std::fs; 

// --- DIPENDENZE CUSTOM OPENSSL (necessarie per reqwest-openssl) ---
// NOTA: Se si usano questi tipi, è necessario che reqwest-openssl e openssl 
// siano definiti nel Cargo.toml del client.
use reqwest::Client as ReqwestClient;
use openssl::ssl::{SslConnector, SslMethod, SslFiletype};
use reqwest_openssl::SslConnectorExt; 


/// Costruisce e restituisce un reqwest::Client configurato per usare HTTPS standard
/// tramite la libreria openssl (per l'uso di certificati custom e la connessione VEN/VTN).
fn build_https_client() -> ReqwestClient {
    // === AGGIUNTA RICHIESTA: Inizializza la libreria OpenSSL ===
    openssl::init(); 

    // 1. CONFIGURAZIONE TLS CLIENT (Standard RSA/ECC, senza Kyber)
    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();

    // Trusta il certificato del server per la verifica (Server's self-signed cert)
    // Questo è il modo OpenSSL per fidarsi del certificato pubblico del VTN.
    builder.set_ca_file("certs/classic/server_cert.pem")
        .expect("❌ ERRORE: Impossibile trovare e fidarsi di certs/classic/server_cert.pem.");

    let ssl_connector = builder.build();

    // 2. Costruisci il client Reqwest usando il connettore custom.
    ReqwestClient::builder()
        // Iniettiamo il nostro SslConnector configurato con il certificato trusted
        .dangerously_use_ssl_connector(ssl_connector)
        .build()
        .expect("Impossibile creare il client Reqwest HTTPS Standard")
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Inizializza il tracing per vedere i log informativi
    tracing_subscriber::fmt::init();

    info!("--- AVVIO SIMULAZIONE VEN HTTPS STANDARD (Dispositivo) ---");

    // 1. COSTRUZIONE CLIENT HTTPS STANDARD CON OPENSSL CUSTOM
    let reqwest_client = build_https_client();

    // Indirizzi del tuo VTN HTTPS
    let vtn_url = "https://vtn:3000/"; 
    let token_url = "https://vtn:3000/auth/token";

    // --- CORREZIONE: GESTIONE ESPLICITA DEGLI ERRORI DI PARSING (URL e ID) ---

    // Conversione e boxing dell'errore di parsing URL
    let vtn_url_parsed: Url = match vtn_url.try_into() {
        Ok(url) => url,
        // Conversione esplicita a Box<dyn Error> per soddisfare la firma di main
        Err(e) => return Err(Box::new(e) as Box<dyn Error>), 
    };
    let token_url_parsed: Url = match token_url.try_into() {
        Ok(url) => url,
        Err(e) => return Err(Box::new(e) as Box<dyn Error>),
    };


    let client = openleadr_client::Client::with_details(
        vtn_url_parsed,
        token_url_parsed,
        reqwest_client, // <-- Client con certificato trusted
        Some(openleadr_client::ClientCredentials::new(
            "admin".to_string(),
            "admin".to_string(),
        )),
    );

    // 2. REGISTRAZIONE VEN
    let random_id = Uuid::new_v4().to_string();
    let program_id_str = "5c3db697-9763-46de-93d8-2944309b9df1";
    
    // Conversione e boxing dell'errore di parsing ID
    let program_id = match program_id_str.parse() {
        Ok(id) => id,
        Err(e) => return Err(Box::new(e) as Box<dyn Error>),
    };
    
    let ven_name = format!("ven-test-{}", &random_id[0..5]);

    info!("Registrazione VEN: '{}'...", ven_name);

    let ven_content = VenContent::new(
        ven_name.clone(), 
        None, 
        None, 
        None
    );

    // L'errore openleadr_client::Error implementa From<Box<dyn Error>>, quindi il ? è sicuro qui.
    let ven_client = client.create_ven(ven_content).await?;
    
    info!("VEN REGISTRATO. ID: {}", ven_client.id());

    // 3. POLLING (LOOP INFINITO SIMULATO)
    info!("\n--- INIZIO POLLING EVENTI ogni 10 secondi ---");
    
    loop {
        let pagination = PaginationOptions { skip: 0, limit: 10 };

        // Il ? qui è sicuro.
        match client.get_events(Some(&program_id), Filter::none(), pagination).await {
            Ok(events) => {
                if events.is_empty() {
                    info!("[{}] Nessun evento. In attesa...", ven_name);
                } else {
                    info!("[{}] ⚡ RICEVUTI {} EVENTI!", ven_name, events.len());
                    for event in events {
                        info!("  -> Event ID: {}", event.id());
                    }
                }
            }
            // Gestione dell'errore di polling. Non usiamo ? per non interrompere il loop.
            Err(e) => {
                error!("❌ Errore nel polling (Connessione o API): {}", e);
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}