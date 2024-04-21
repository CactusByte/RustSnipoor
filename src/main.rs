use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use reqwest;
use chrono::Local;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Solana WebSocket URL for subscribing to logs.
    let solana_ws_url = "wss://endpoint_ws_provider";
    let wss_url = Url::parse(solana_ws_url)?;

    // Connect to the Solana WebSocket endpoint.
    let (mut ws_stream, _) = connect_async(wss_url).await.expect("Failed to connect");

    // Construct the subscription message for logs mentioning a specific program ID.
    // Adjust the filter as needed for your specific use case.
    let subscribe_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            {
                "mentions": ["675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"], // Replace with your program ID
            },
            {
                "encoding": "jsonParsed",
                "commitment": "finalized"
            }
        ]
    })
    .to_string();

    // Send the subscription message to subscribe to logs.
    ws_stream.send(Message::Text(subscribe_message)).await?;

    // Listen for messages (logs) from the WebSocket stream.
    while let Some(message) = ws_stream.next().await {
        let now = Local::now();
                
        match message {
            Ok(Message::Text(text)) => {
                // Attempt to parse the message as JSON.
                if let Ok(log) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Check if the log message contains "initialize2".
                    if log["params"]["result"]["value"]["logs"]
                    .as_array()
                    .map_or(false, |logs| logs.iter().any(|log| log.as_str().map_or(false, |s| s.contains("initialize2"))))
                {
                    println!("Transaction received at: {}", now.format("%H:%M:%S"));
                    let tx_signature = log["params"]["result"]["value"]["signature"].as_str().unwrap_or_default();
                    println!("Detected 'initialize2' log: https://solscan.io/tx/{}", tx_signature);
                    if let Ok(transaction_details) = fetch_transaction_details(tx_signature).await {
                        if let Ok(addresses) = print_mint_addresses(&transaction_details) {
                            if let (Some(input_token), Some(output_token)) = (addresses.get(0), addresses.get(1)) {
                                // Now you have `input_token` and `output_token` as `Some(&String)`
                                // Convert them to `&str` with `as_str()` for the function call
                                println!("{}, {}", input_token, output_token)
                                // Note: You might need to adjust the get_swap_quote function or this call based on async context
                            }
                        }
                    }
                    println!();
                }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn fetch_transaction_details(transaction_signature: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let solana_rpc_url = "https: https_provider"; // Adjust for mainnet or testnet if necessary
    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}
        ]
    });

    let response = client.post(solana_rpc_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let response_json = response.json::<Value>().await?;
    println!("Transaction details fetched successfully.");
    Ok(response_json)
}

fn print_mint_addresses(transaction_details: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {

    let empty_vec = Vec::new();
    let mut mint_addresses = Vec::new();

    let inner_instructions = transaction_details["result"]["meta"]["innerInstructions"]
        .as_array()
        .unwrap_or(&empty_vec);

    for instruction_set in inner_instructions {
        for instruction in instruction_set["instructions"].as_array().unwrap_or(&Vec::new()) {
            let parsed = &instruction["parsed"];
            let program = instruction["program"].as_str().unwrap_or_default();
            let instruction_type = parsed["type"].as_str().unwrap_or_default();

            if program == "spl-token" && instruction_type == "initializeAccount" {
                let info = &parsed["info"];
                let mint_address = info["mint"].as_str().unwrap_or_default();
                //println!("Mint address found: {}", mint_address);
                if !mint_address.is_empty() {
                    mint_addresses.push(mint_address.to_string());
                }
            }
        }
    }
    
    for address in &mint_addresses {
        println!("Mint address found: {}", address);
    }

    Ok(mint_addresses)
}
