# request_manager

## Usage Example

```bash

cargo build --release
cargo run --release

```

# Run the application using docker

## Requirements
- Docker 26.1.3

## Build the Docker Image
Run the following command to build the Docker image:
```bash
docker build -t request_manager -f Dockerfile.request-manager .
```
## Run the Docker Container
Run the Docker container with the following command, passing the .env environment variable:
```bash
docker run --env-file .env request_manager
```