use crate::data_loader::HistoricalQuote;
use crate::strategy::{AdaptiveEngine, Signal, Strategy};
use std::collections::HashMap;

pub fn run_backtest(quotes: Vec<HistoricalQuote>) {
    let mut engine = AdaptiveEngine::new();
    let mut balance: f64 = 2500.0;
    
    let mut current_quotes: HashMap<String, HistoricalQuote> = HashMap::new();
    let mut total_trades: u64 = 0;
    let mut pnl_list: Vec<f64> = Vec::new();
    let mut entry_price: f64 = 0.0;
    let mut entry_size: f64 = 0.0;
    let mut current_drawdown: f64 = 0.0;
    let mut peak_balance: f64 = balance;

    for quote in quotes {
        current_quotes.insert(quote.symbol.clone(), quote);

        if current_quotes.len() < 2 {
            continue;
        }

        let q_a = current_quotes.get("AAPL").unwrap();
        let q_b = current_quotes.get("MSFT").unwrap();

        let spread_price = q_a.ask_price - q_b.bid_price; 

        let action = engine.on_tick(
            q_a.ask_price,
            q_b.bid_price,
            (q_a.ask_size + q_a.bid_size) / 2.0,
            (q_b.ask_size + q_b.bid_size) / 2.0,
            1, 1, 
            balance,
            0 
        );

        if action.size > 0.0 {
            match action.signal {
                Signal::Buy | Signal::Sell => {
                    let cost = (q_a.ask_price + q_b.bid_price) * action.size * action.execution_slippage;
                    balance -= cost;
                    entry_price = spread_price;
                    entry_size = action.size;
                    total_trades += 1;
                }
                Signal::Close => {
                    let exit_price = spread_price;
                    let pnl = (exit_price - entry_price) * entry_size;
                    balance += pnl - (exit_price * entry_size * action.execution_slippage);
                    pnl_list.push(pnl);
                    
                    if balance > peak_balance {
                        peak_balance = balance;
                    } else {
                        current_drawdown = current_drawdown.max(peak_balance - balance);
                    }
                }
                _ => {}
            }
        }
    }

    println!("--- Final Report ---");
    println!("Total Trades: {}", total_trades);
    println!("Net PnL: {:.2}", pnl_list.iter().sum::<f64>());
    println!("Max Drawdown: {:.2}", current_drawdown);
    println!("Final Account Balance: {:.2}", balance);
}
