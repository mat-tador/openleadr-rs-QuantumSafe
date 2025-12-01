use openleadr_client::{Client, ClientCredentials, Filter, PaginationOptions}; // <--- AGGIUNTI Filter e PaginationOptions
use openleadr_wire::ven::VenContent;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- AVVIO SIMULAZIONE VEN (Dispositivo) ---");

    // 1. CONNESSIONE
    let reqwest_client = reqwest::Client::new();
    let client = Client::with_details(
        "https://vtn:3000/".try_into()?,
        "https://vtn:3000/auth/token".try_into()?,
        reqwest_client,
        Some(ClientCredentials::new(
            "admin".to_string(),
            "admin".to_string(),
        )),
    );

    // 2. REGISTRAZIONE VEN
    let random_id = Uuid::new_v4().to_string();
    let program_id_str = "5c3db697-9763-46de-93d8-2944309b9df1";
    let program_id = program_id_str.parse()?;
    let ven_name = format!("ven-test-{}", &random_id[0..5]);

    println!("Registrazione VEN: '{}'...", ven_name);

    // Creiamo il VEN (con i 4 parametri richiesti: nome + 3 None)
    let ven_content = VenContent::new(
        ven_name.clone(), 
        None, 
        None, 
        None
    );

    let ven_client = client.create_ven(ven_content).await?;
    println!("VEN REGISTRATO. ID: {}", ven_client.id());

    // 3. POLLING (LOOP INFINITO)
    println!("\n--- INIZIO POLLING EVENTI ---");
    
    loop {
        // CORREZIONE: Usiamo "client" (il principale), non "ven_client".
        // Chiediamo: "Dammi tutti gli eventi" (Filter::none())
        // Con paginazione: i primi 10
        let pagination = PaginationOptions { skip: 0, limit: 10 };

        // Nota: la funzione potrebbe chiamarsi "get_events_request" o "get_events" 
        // a seconda della versione esatta, ma dai tuoi file precedenti sembra "get_events_request"
        match client.get_events(Some(&program_id), Filter::none(), pagination).await {
            Ok(events) => {
                if events.is_empty() {
                    println!("[{}] Nessun evento. In attesa...", ven_name);
                } else {
                    println!("[{}] ⚡ RICEVUTI {} EVENTI!", ven_name, events.len());
                    for event in events {
                        println!("   -> Event ID: {}", event.id());
                        // Qui puoi leggere i dettagli dell'evento
                    }
                }
            }
            Err(e) => {
                eprintln!("Errore nel polling: {}", e);
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}