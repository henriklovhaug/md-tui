# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.77.2

################################################################################
# Create a stage for building the application.
FROM --platform=linux/amd64 rust:${RUST_VERSION}-alpine AS build
WORKDIR /app

# Install host build dependencies.
RUN apk add --no-cache clang lld musl-dev git file

RUN cargo install cargo-aur

RUN mkdir -p /usr/final/target

COPY . .

RUN cargo-aur && \
  cp ./target/cargo-aur/* /usr/final/target/

FROM scratch AS final

# Copy the executable from the "build" stage.
COPY --from=build /usr/final/target /bin/


ENTRYPOINT [ "/bin/mdt" ]

# Build command
# docker build --output=bin -t mdt .

