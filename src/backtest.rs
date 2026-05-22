use crate::data_loader::HistoricalQuote;
use crate::strategy::{AdaptiveEngine, Signal, Strategy};
use std::collections::HashMap;
use csv::Writer;

pub fn run_backtest(quotes: Vec<HistoricalQuote>, pairs: &[(&str, &str)]) {
    for &(sym_a, sym_b) in pairs {
        println!("--- Backtesting pair: {}/{} ---", sym_a, sym_b);
        let mut engine = AdaptiveEngine::new();
        let mut balance: f64 = 100.0;
        let mut current_state: i8 = 0;
        
        let mut current_quotes: HashMap<String, HistoricalQuote> = HashMap::new();
        let mut total_trades: u64 = 0;
        let mut pnl_list: Vec<f64> = Vec::new();
        let mut entry_price: f64 = 0.0;
        let mut entry_size: f64 = 0.0;
        let mut entry_time: String = String::new();
        let mut current_drawdown: f64 = 0.0;
        let mut peak_balance: f64 = 100.0;

        let log_filename = format!("trade_log_{}_{}.csv", sym_a, sym_b);
        let mut wtr = Writer::from_path(log_filename).unwrap();
        wtr.write_record(&["Entry Time", "Exit Time", "Side", "Entry Spread", "Exit Spread", "Size", "PnL"]).unwrap();

        for quote in quotes.iter().filter(|q| q.symbol == sym_a || q.symbol == sym_b) {
            current_quotes.insert(quote.symbol.clone(), quote.clone());

            if current_quotes.len() < 2 {
                continue;
            }

            let q_a = current_quotes.get(sym_a).unwrap();
            let q_b = current_quotes.get(sym_b).unwrap();

            let spread_price = q_a.ask_price - q_b.bid_price; 

            let action = engine.on_tick(
                q_a.ask_price,
                q_b.bid_price,
                (q_a.ask_size + q_a.bid_size) / 2.0,
                (q_b.ask_size + q_b.bid_size) / 2.0,
                1, 1, 
                balance,
                current_state
            );

            match action.signal {
                Signal::Buy if action.size > 0.0 => {
                    let cost = (q_a.ask_price + q_b.bid_price) * action.size * action.execution_slippage;
                    balance -= cost;
                    entry_price = spread_price;
                    entry_size = action.size;
                    entry_time = q_a.timestamp.clone();
                    total_trades += 1;
                    current_state = 1;
                }
                Signal::Sell if action.size > 0.0 => {
                    let cost = (q_a.ask_price + q_b.bid_price) * action.size * action.execution_slippage;
                    balance -= cost;
                    entry_price = spread_price;
                    entry_size = action.size;
                    entry_time = q_a.timestamp.clone();
                    total_trades += 1;
                    current_state = -1;
                }
                Signal::Close if current_state != 0 => {
                    let exit_price = spread_price;
                    let exit_time = q_a.timestamp.clone();
                    let side = if current_state == 1 { "Long" } else { "Short" };
                    let pnl = if current_state == 1 {
                        (exit_price - entry_price) * entry_size
                    } else {
                        (entry_price - exit_price) * entry_size
                    };
                    
                    balance += pnl - (exit_price * entry_size * action.execution_slippage);
                    pnl_list.push(pnl);
                    
                    wtr.write_record(&[
                        entry_time.clone(),
                        exit_time,
                        side.to_string(),
                        entry_price.to_string(),
                        exit_price.to_string(),
                        entry_size.to_string(),
                        pnl.to_string()
                    ]).unwrap();

                    current_state = 0;
                    
                    if balance > peak_balance {
                        peak_balance = balance;
                    } else {
                        current_drawdown = current_drawdown.max(peak_balance - balance);
                    }
                }
                _ => {}
            }
        }
        wtr.flush().unwrap();

        println!("--- Final Report for {}/{} ---", sym_a, sym_b);
        println!("Total Trades: {}", total_trades);
        println!("Net PnL: {:.2}", pnl_list.iter().sum::<f64>());
        println!("Max Drawdown: {:.2}", current_drawdown);
        println!("Final Account Balance: {:.2}", balance);
    }
}
