mod strategy;

use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
struct Account {
    cash: String,
    buying_power: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let api_key = env::var("ALPACA_API_KEY").expect("ALPACA_API_KEY must be set");
    let secret_key = env::var("ALPACA_SECRET_KEY").expect("ALPACA_SECRET_KEY must be set");
    let base_url = env::var("ALPACA_BASE_URL").unwrap_or_else(|_| "https://paper-api.alpaca.markets".to_string());

    let client = reqwest::Client::new();
    let account_url = format!("{}/v2/account", base_url.trim_end_matches("/v2"));

    println!("Fetching Alpaca account status...");

    let response = client
        .get(account_url)
        .header("APCA-API-KEY-ID", api_key)
        .header("APCA-API-SECRET-KEY", secret_key)
        .send()
        .await?
        .error_for_status()?;

    let account: Account = response.json().await?;

    println!("Successfully connected to Alpaca!");
    println!("Cash Balance: ${}", account.cash);
    println!("Buying Power: ${}", account.buying_power);

    Ok(())
}
