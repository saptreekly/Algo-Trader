use algo_trader::data_loader::load_historical_quotes;
use algo_trader::backtest::run_backtest;

fn main() {
    let quotes_path = "data/mock_quotes_aapl_msft.csv";
    match load_historical_quotes(quotes_path) {
        Ok(quotes) => {
            run_backtest(quotes);
        }
        Err(e) => {
            eprintln!("Failed to load data: {}", e);
        }
    }
}
