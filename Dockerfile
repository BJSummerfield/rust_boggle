# Stage 1: Build with musl libc
FROM rust:latest as builder
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/boggle_game

# Copy the source code and static files
COPY ./src ./src
COPY ./static ./static
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Build the application in release mode targeting musl
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Use Alpine for the runtime environment
FROM alpine:latest

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/boggle_game/target/x86_64-unknown-linux-musl/release/boggle_game /usr/local/bin/

# Copy the static files from the builder stage
COPY --from=builder /usr/src/boggle_game/static /app/static

# Expose the port the application listens on
EXPOSE 3000

# Set the command to run the application
CMD ["boggle_game"]
