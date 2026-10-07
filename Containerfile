FROM docker.io/library/ubuntu:26.04 AS build

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /work
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --locked --release

FROM docker.io/library/ubuntu:26.04
COPY --from=build /work/target/release/jett /usr/local/bin/jett
USER 1000:1000
WORKDIR /tmp
ENTRYPOINT ["jett"]
