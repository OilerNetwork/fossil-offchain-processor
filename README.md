# fossil-offchain-processor

## Running the backend

```bash

cargo run --release

```

## APIs

```bash

# POST /   -- call_mev_blocker_api

curl --location 'http://localhost:3000/' \
--header 'Content-Type: application/json' \
--data '{
	"account_address":"0x7F0d15C7FAae65896648C8273B6d7E43f58Fa842",
    "storage_keys": [  "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421" ]
}'

```

## Tests

For running all the test cases

```bash

cargo test --release

```

To run a specific test case `test_name`

```bash

cargo test test_name

```
