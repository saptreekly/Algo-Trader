use algo_trader::data_loader::load_historical_quotes;
use algo_trader::backtest::run_backtest;

fn main() {
    let quotes_path = "data/mag7_alpaca_quotes.csv";
    let pairs = [("AAPL", "MSFT"), ("GOOGL", "AMZN"), ("META", "NVDA"), ("AAPL", "TSLA"), ("MSFT", "NVDA")];
    
    match load_historical_quotes(quotes_path) {
        Ok(quotes) => {
            run_backtest(quotes, &pairs);
        }
        Err(e) => {
            eprintln!("Failed to load data: {}", e);
        }
    }
}
