use openleadr_client::{Client, ClientCredentials, Filter, PaginationOptions};
use openleadr_wire::ven::VenContent;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::hash::MessageDigest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- AVVIO SIMULAZIONE VEN (Dispositivo Sicuro) ---");

    // --- 0. PREPARAZIONE CRITTOGRAFIA (Firma Dati) ---
    // Carichiamo la chiave privata ECC del client per firmare i messaggi
    println!("🔐 Caricamento chiave ECC per firma dati...");
    let key_pem = fs::read("certs/classic/client_data_key.pem")?;
    let keypair = PKey::private_key_from_pem(&key_pem)?;
    
    // Eseguiamo una firma di prova (Benchmark Application Layer)
    let dati_da_firmare = b"Messaggio Importante dal VEN";
    let mut signer = Signer::new(MessageDigest::sha256(), &keypair)?;
    signer.update(dati_da_firmare)?;
    let firma = signer.sign_to_vec()?;
    println!("✍️  Firma ECC generata ({} bytes): {:X?}", firma.len(), &firma[0..10]);


    // --- 1. CONNESSIONE HTTPS ---
    // Carichiamo la CA per fidarci del server (Certificate Pinning)
    let ca_cert_pem = fs::read("certs/classic/ca_cert.pem")?;
    let ca_cert = reqwest::Certificate::from_pem(&ca_cert_pem)?;

    // Costruiamo il client HTTP sicuro
    let reqwest_client = reqwest::Client::builder()
        .use_native_tls() // Usa la nostra OpenSSL custom
        .add_root_certificate(ca_cert) // Fidati della nostra CA
        .build()?;

    let client = Client::with_details(
        "https://vtn:3000/".try_into()?, // Nota: localhost per test locale
        "https://vtn:3000/auth/token".try_into()?,
        reqwest_client,
        Some(ClientCredentials::new(
            "admin".to_string(),
            "admin".to_string(),
        )),
    );

    // 2. REGISTRAZIONE VEN
    let random_id = Uuid::new_v4().to_string();
    let ven_name = format!("ven-secure-{}", &random_id[0..5]);

    println!("📡 Tentativo di connessione HTTPS a VTN...");
    
    // Creiamo il VEN
    let ven_content = VenContent::new(ven_name.clone(), None, None, None);

    match client.create_ven(ven_content).await {
        Ok(ven_client) => {
            println!("✅ VEN REGISTRATO SU HTTPS! ID: {}", ven_client.id());
            // Qui in futuro invieremo la 'firma' come parte di un report
        },
        Err(e) => {
            eprintln!("❌ Errore connessione: {}. Il certificato è valido?", e);
            return Ok(());
        }
    }

    // 3. POLLING
    let program_id_str = "5c3db697-9763-46de-93d8-2944309b9df1";
    // Gestiamo il caso in cui il program ID non esista ancora
    let program_id = match program_id_str.parse() {
        Ok(id) => Some(id),
        Err(_) => None
    };

    println!("\n--- INIZIO POLLING EVENTI (Canale Cifrato) ---");
    loop {
        let pagination = PaginationOptions { skip: 0, limit: 10 };
        
        match client.get_events(program_id.as_ref(), Filter::none(), pagination).await {
            Ok(events) => {
                if events.is_empty() {
                    println!("[{}] Nessun evento.", ven_name);
                } else {
                    println!("[{}] ⚡ RICEVUTI {} EVENTI!", ven_name, events.len());
                }
            }
            Err(e) => eprintln!("Errore polling: {}", e),
        }
        sleep(Duration::from_secs(10)).await;
    }
}