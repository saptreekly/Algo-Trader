import csv
import random
from datetime import datetime, timedelta

def generate_mock_data(filename, num_rows=10000):
    aapl_price = 150.0
    msft_price = 250.0
    start_time = datetime(2026, 5, 22, 9, 30)

    with open(filename, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['timestamp', 'symbol', 'bid_price', 'bid_size', 'ask_price', 'ask_size'])

        for i in range(num_rows):
            # Co-integration drift
            drift = random.gauss(0, 0.05)
            aapl_price += drift + random.gauss(0, 0.02)
            msft_price += drift + random.gauss(0, 0.03)

            symbol = 'AAPL' if i % 2 == 0 else 'MSFT'
            price = aapl_price if symbol == 'AAPL' else msft_price

            # Spread volatility
            spread = random.uniform(0.01, 0.05)
            if random.random() < 0.05:  # Volatility spike
                spread *= 3

            bid = price - spread / 2
            ask = price + spread / 2
            
            timestamp = (start_time + timedelta(seconds=i)).isoformat()
            
            writer.writerow([
                timestamp,
                symbol,
                round(bid, 2),
                random.randint(100, 5000),
                round(ask, 2),
                random.randint(100, 5000)
            ])

generate_mock_data('data/mock_quotes_aapl_msft.csv')
