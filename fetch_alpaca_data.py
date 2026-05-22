import os
import csv
from datetime import datetime
from dotenv import load_dotenv
from alpaca.data.historical import StockHistoricalDataClient
from alpaca.data.requests import StockQuotesRequest
from tqdm import tqdm

# Load credentials from .env file
load_dotenv()

# Ensure you have installed: pip install alpaca-py python-dotenv tqdm
API_KEY = os.environ.get('ALPACA_API_KEY')
SECRET_KEY = os.environ.get('ALPACA_SECRET_KEY')

if not API_KEY or not SECRET_KEY:
    raise ValueError("ALPACA_API_KEY and ALPACA_SECRET_KEY must be set in your .env file.")

client = StockHistoricalDataClient(API_KEY, SECRET_KEY)

# The Magnificent 7
mag7_symbols = ["AAPL", "MSFT", "GOOGL", "AMZN", "META", "NVDA", "TSLA"]

# Let's grab 2 hours of intense morning volatility 
start_time = datetime(2024, 5, 15, 13, 30) # 9:30 AM EST in UTC
end_time = datetime(2024, 5, 15, 15, 30)   # 11:30 AM EST in UTC

request_params = StockQuotesRequest(
    symbol_or_symbols=mag7_symbols,
    start=start_time,
    end=end_time
)

print("Fetching historical quotes from Alpaca...")
quotes = client.get_stock_quotes(request_params)

output_file = 'data/mag7_alpaca_quotes.csv'
os.makedirs('data', exist_ok=True)

# Calculate total quotes for tqdm
total_quotes = sum(len(q_list) for q_list in quotes.data.values())

with open(output_file, 'w', newline='') as f:
    writer = csv.writer(f)
    writer.writerow(['timestamp', 'symbol', 'bid_price', 'bid_size', 'ask_price', 'ask_size'])

    # Use tqdm to track progress
    with tqdm(total=total_quotes, desc="Writing quotes to CSV") as pbar:
        for symbol, quote_list in quotes.data.items():
            for q in quote_list:
                writer.writerow([
                    q.timestamp.isoformat(),
                    symbol,
                    round(float(q.bid_price), 2),
                    float(q.bid_size),
                    round(float(q.ask_price), 2),
                    float(q.ask_size)
                ])
                pbar.update(1)

print(f"Success! Data saved to {output_file}")