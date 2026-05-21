use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub async fn submit_order(
    symbol: &str,
    qty: f64,
    side: &str,
    client: &Client,
    api_key: &str,
    api_secret: &str,
    base_url: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/v2/orders", base_url);
    let body = json!({
        "symbol": symbol,
        "qty": qty,
        "side": side,
        "type": "market",
        "time_in_force": "day"
    });

    client
        .post(url)
        .header("APCA-API-KEY-ID", api_key)
        .header("APCA-API-SECRET-KEY", api_secret)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    
    Ok(())
}
