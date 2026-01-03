# Builder stage
FROM rust:1.83-bookworm AS builder

WORKDIR /usr/src/cronet-cloak

# Install build dependencies
# - protobuf-compiler for prost
# - libclang-dev for bindgen
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libclang-dev \
    libnss3-dev \
    libnspr4-dev \
    libglib2.0-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Debug: Check file existence
RUN ls -R cronet-bin

# Build for release
RUN export MAJOR=$(grep MAJOR cronet-bin/linux/VERSION | cut -d= -f2) && \
    export MINOR=$(grep MINOR cronet-bin/linux/VERSION | cut -d= -f2) && \
    export BUILD=$(grep BUILD cronet-bin/linux/VERSION | cut -d= -f2) && \
    export PATCH=$(grep PATCH cronet-bin/linux/VERSION | cut -d= -f2) && \
    export CRONET_VERSION="$MAJOR.$MINOR.$BUILD.$PATCH" && \
    ln -s libcronet.so cronet-bin/linux/libcronet.$CRONET_VERSION.so

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies required by Cronet
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libstdc++6 \
    libgcc-s1 \
    libglib2.0-0 \
    libnss3 \
    libnspr4 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /usr/src/cronet-cloak/target/release/cronet-cloak .

# Copy Cronet shared library
COPY --from=builder /usr/src/cronet-cloak/cronet-bin/linux/libcronet.so /usr/local/lib/
COPY --from=builder /usr/src/cronet-cloak/cronet-bin/linux/VERSION /usr/local/lib/

# Create versioned symlink because SONAME expects it
RUN export MAJOR=$(grep MAJOR /usr/local/lib/VERSION | cut -d= -f2) && \
    export MINOR=$(grep MINOR /usr/local/lib/VERSION | cut -d= -f2) && \
    export BUILD=$(grep BUILD /usr/local/lib/VERSION | cut -d= -f2) && \
    export PATCH=$(grep PATCH /usr/local/lib/VERSION | cut -d= -f2) && \
    export CRONET_VERSION="$MAJOR.$MINOR.$BUILD.$PATCH" && \
    ln -s libcronet.so /usr/local/lib/libcronet.$CRONET_VERSION.so && \
    ldconfig /usr/local/lib
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Expose port (Rust app uses 3000 by default, previously 8080)
# Go project used port 80 and ENV SVC_PORT=80.
# Assuming we stick to Rust default 3000 for now or user configures it.
EXPOSE 3000

ENV RUST_LOG=info

CMD ["./cronet-cloak"]
