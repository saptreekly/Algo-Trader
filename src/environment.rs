use std::collections::HashMap;
use crate::data_loader::CsvQuote;

pub struct TradingEnvironment {
    pub assets: HashMap<String, Vec<CsvQuote>>,
    pub active_symbol: Option<String>,
    pub current_step: usize,
    pub position_state: i32,
    pub entry_price: f32,
    pub equity: f32,
    pub price_history: Vec<f32>,
}

impl TradingEnvironment {
    pub fn new(assets: HashMap<String, Vec<CsvQuote>>) -> Self {
        Self {
            assets,
            active_symbol: None,
            current_step: 0,
            position_state: 0,
            entry_price: 0.0,
            equity: 10000.0,
            price_history: Vec::new(),
        }
    }

    pub fn load_ticker_session(&mut self, symbol: &str) -> Result<(), String> {
        if self.assets.contains_key(symbol) {
            self.active_symbol = Some(symbol.to_string());
            self.current_step = 0;
            self.position_state = 0;
            self.entry_price = 0.0;
            self.equity = 10000.0;
            self.price_history = Vec::new();
            Ok(())
        } else {
            Err("Ticker not found".to_string())
        }
    }

    fn get_std(data: &[f32], mean: f32) -> f32 {
        let n = data.len() as f32;
        (data.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n).sqrt()
    }

    fn compute_state_vector(&self) -> Vec<f32> {
        let n = self.price_history.len();
        if n < 100 { return vec![0.0; 8]; }

        let mid = self.price_history[n - 1];
        let ret1 = (mid / self.price_history[n - 2]).ln();
        let ret10 = (mid / self.price_history[n - 11]).ln();

        let window50 = &self.price_history[n-50..n];
        let mean50 = window50.iter().sum::<f32>() / 50.0;
        let std50 = Self::get_std(window50, mean50);
        let zscore = if std50 > 0.0 { (mid - mean50) / std50 } else { 0.0 };

        let std10 = Self::get_std(&self.price_history[n-10..n], self.price_history[n-10..n].iter().sum::<f32>() / 10.0);
        let std100 = Self::get_std(&self.price_history[n-100..n], self.price_history[n-100..n].iter().sum::<f32>() / 100.0);
        let vol_ratio = if std100 > 0.0 { std10 / std100 } else { 1.0 };

        let quote = &self.assets[self.active_symbol.as_ref().unwrap()][self.current_step - 1];
        let imbalance = (quote.bid_size - quote.ask_size) / (quote.bid_size + quote.ask_size + 1e-6);
        let spread = (quote.ask_price - quote.bid_price) / mid;

        let pos = self.position_state as f32;
        let pnl = if self.position_state != 0 { (mid - self.entry_price) / self.entry_price } else { 0.0 };

        vec![ret1, ret10, zscore, vol_ratio, imbalance, spread, pos, pnl]
    }

    pub fn step(&mut self, action: usize) -> (Vec<f32>, f32, bool) {
        let quotes = &self.assets[self.active_symbol.as_ref().unwrap()];
        let quote = &quotes[self.current_step];
        self.price_history.push((quote.bid_price + quote.ask_price) / 2.0);
        
        let reward = 0.0;
        let mid = (quote.bid_price + quote.ask_price) / 2.0;
        match action {
            1 => { if self.position_state == 0 { self.position_state = 1; self.entry_price = mid; } },
            2 => { if self.position_state == 0 { self.position_state = -1; self.entry_price = mid; } },
            _ => {}
        }
        
        self.current_step += 1;
        let is_done = self.current_step >= quotes.len();
        (self.compute_state_vector(), reward, is_done)
    }
}
