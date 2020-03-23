FROM rust:latest

ARG uid
ARG gid

RUN (addgroup runner --gid $gid || continue) \
    && adduser --disabled-password --uid $uid --gid $gid --home /home/runner runner \
    && chown -R $uid:$gid /home/runner

RUN apt-get update && apt-get install -y sudo curl && rustup update && rustup component add rustfmt clippy \
    && cargo install sccache && chown -R $uid:$gid /usr/local/cargo /usr/local/rustup

USER runner
