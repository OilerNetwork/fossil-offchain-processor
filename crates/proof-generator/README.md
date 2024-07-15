# request-mananer

## Running the backend

```bash

cargo run --release

```

## Run the application using docker

### Requirements
- Docker 26.1.3

### Build the Docker Image
Run the following command to build the Docker image:
```bash
docker build -t proof_generator .
```
### Run the Docker Container
Run the Docker container with the following command, passing the .env environment variable:
```bash
docker run --env-file .env proof_generator
```

## APIs

```bash

# POST /   -- call_mev_blocker_api

curl --location 'http://localhost:3000/' \
--header 'Content-Type: application/json' \
--data '{
	"account_address":"0x6b175474e89094c44da98b954eedeac495271d0f",
    "storage_keys": [  "0x199c2e6b850bcc9beaea25bf1bacc5741a7aad954d28af9b23f4b53f5404937b" ]
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
