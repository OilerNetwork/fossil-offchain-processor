# Fossil v0.3

![Fossil v0.3 components interaction](./readme/Fossil%20v0.3%20components%20interaction.png)

![Fossil v0.3 sequence diagram](./readme/Fossil%20v0.3%20sequence%20diagram.png)

## Usage

```bash
cargo run --bin server
```

### Example request

```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: c4ba7033-46a3-4ce7-b39c-ddfe4a1af8bb" \
  -d '{
    "identifiers": ["0x50495443485f4c414b455f5631"],
    "params": {
      "twap": [1672531200, 1672574400],
      "volatility": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    },
    "client_info": {
      "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
      "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
      "timestamp": 1741243059
    }
  }'
```
