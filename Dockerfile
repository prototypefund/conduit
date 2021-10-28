# syntax=docker/dockerfile:1
FROM rust:1.53-alpine as builder
WORKDIR /usr/src/conduit

# == Build dependencies without our own code separately for caching ==
#
# Need a fake main.rs since Cargo refuses to build anything otherwise.
#
# See https://github.com/rust-lang/cargo/issues/2644 for a Cargo feature
# request that would allow just dependencies to be compiled, presumably
# regardless of whether source files are available.
RUN mkdir src && echo 'fn main() {}' > src/main.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build
# TODO: RUN cargo build --release

# == Actual build ==
RUN rm -r src
COPY src src
# main.rs has to have its timestamp updated for this to work correctly since
# otherwise the build with the fake main.rs from above is newer than the
# source files (COPY preserves timestamps).
RUN touch src/main.rs

RUN cargo install --path .
# TODO: RUN cargo install --release --path . 


# This build stage is going to be run later
FROM alpine:3.14

# Install packages needed to run Conduit
RUN apk add --no-cache \
        ca-certificates \
        curl \
        libgcc

# Prepare path for database and media files
RUN mkdir -p /srv/conduit/.local/share/conduit

# TODO: Change ? or maybe leave it like that
RUN mkdir -p /srv/conduit/.local/share/conduit
COPY --from=builder /usr/local/cargo/bin/conduit /srv/conduit/

# TODO: Check if we don't want to just use ENVs for running condit in docker
ENV CONDUIT_CONFIG="/srv/conduit/conduit.toml"

# TODO: not needed, but documents it?
EXPOSE 6167

WORKDIR /srv/conduit
ENTRYPOINT [ "/srv/conduit/conduit" ]