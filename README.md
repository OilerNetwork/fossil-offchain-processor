# Fossil v0.3

## Development

### Setup

To set up the development environment, run:

```bash
make setup
```

This will install all required dependencies, including Rust nightly.

### Building

To build the project in release mode:

```bash
make build
```

For debug mode:

```bash
make build-debug
```

### Testing

Run all tests:

```bash
make test
```

### Linting

Format code and run all linters:

```bash
make lint
```

Individual linting commands:

```bash
make fmt        # Format code with rustfmt
make clippy     # Run clippy linter
make codespell  # Check for spelling mistakes
```

### Pull Request Preparation

Before submitting a PR, run:

```bash
make pr
```

This will run all linters and tests to ensure your code is ready for review.

For more available commands:

```bash
make help
```

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
      "cap_level": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
      "alpha": 1234,
      "k": -1234,
    },
    "client_info": {
      "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
      "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
      "timestamp": 1741243059
    }
  }'
```
