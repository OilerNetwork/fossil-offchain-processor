# Fossil v0.3

![Fossil v0.3 components interaction](./readme/Fossil%20v0.3%20components%20interaction.png)

![Fossil v0.3 sequence diagram](./readme/Fossil%20v0.3%20sequence%20diagram.png)

## Usage

```bash
cargo run --release
```

### Example request

```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: c4ba7033-46a3-4ce7-b39c-ddfe4a1af8bb" \
  -d '{
    "identifiers": ["ETH"],
    "params": {
      "twap": [1672531200, 1672574400],
      "volatility": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    },
    "client_info": {
      "client_address": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "vault_address": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "timestamp": 1672574400
    }
  }'
```
