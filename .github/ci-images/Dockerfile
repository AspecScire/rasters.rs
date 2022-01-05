ARG GDAL_VERSION=3.2.2
ARG RUST_VERSION=1.57

FROM rust:${RUST_VERSION} as rust

FROM osgeo/gdal:ubuntu-full-${GDAL_VERSION} as runner

COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get \
    install -y --fix-missing --no-install-recommends \
    build-essential
