# Use the official Rust image.
FROM rust:latest

# Set the working directory inside the container.
WORKDIR /usr/src/eth-dispatcher

# Copy the submodule Rust project code to the container's working directory.
COPY . ./

# Build dependencies first - useful for caching.
RUN cargo fetch

# Build your project.
RUN cargo build --release

# Command to run the executable using `cargo run`.
# Note: `cargo run` compiles the debug version, adjust as needed.
CMD ["cargo", "run", "--example", "relayer"]