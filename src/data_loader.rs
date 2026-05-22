use serde::Deserialize;
use std::error::Error;
use std::fs::File;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuote {
    pub timestamp: String, // Or use chrono::DateTime<Utc> if preferred
    pub symbol: String,
    pub bid_price: f64,
    pub bid_size: f64,
    pub ask_price: f64,
    pub ask_size: f64,
}

pub fn load_historical_quotes(file_path: &str) -> Result<Vec<HistoricalQuote>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut quotes = Vec::new();

    for result in rdr.deserialize() {
        let quote: HistoricalQuote = result?;
        quotes.push(quote);
    }

    Ok(quotes)
}
