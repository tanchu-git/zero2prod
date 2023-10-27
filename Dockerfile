FROM rust:1.73.0

# Set working directory to `app` (equivalent to `cd app`)
WORKDIR /app

# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y

# Copy all files from our working environment to our Docker image
COPY . .

ENV SQLX_OFFLINE=true

# Let's build our binary
RUN cargo build --release

ENV APP_ENVIRONMENT production

# When `docker run` is executed, launch the binary
ENTRYPOINT ["./target/release/zero2prod"]
