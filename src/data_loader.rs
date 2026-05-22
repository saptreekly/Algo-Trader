use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;

#[derive(Debug, Deserialize, Clone)]
pub struct CsvQuote {
    pub timestamp: String,
    pub symbol: String,
    pub bid_price: f32,
    pub bid_size: f32,
    pub ask_price: f32,
    pub ask_size: f32,
}

pub fn load_universe_data(file_path: &str) -> Result<HashMap<String, Vec<CsvQuote>>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_reader(File::open(file_path)?);
    let mut data: HashMap<String, Vec<CsvQuote>> = HashMap::new();

    for result in reader.deserialize() {
        let quote: CsvQuote = result?;
        data.entry(quote.symbol.clone()).or_insert_with(Vec::new).push(quote);
    }

    for quotes in data.values_mut() {
        quotes.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    Ok(data)
}
