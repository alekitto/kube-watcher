# syntax=docker/dockerfile:1.9

FROM bitnami/minideb:bookworm AS build

ENV PATH=/root/.cargo/bin:$PATH

RUN <<EOF /bin/bash
  set -eux
  install_packages curl ca-certificates libc6-dev libssl-dev make g++ pkg-config
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
EOF

ADD . /opt/src
WORKDIR /opt/src

RUN cargo build --release
RUN cp target/release/kube-watcher /kube-watcher

FROM bitnami/minideb:bookworm AS stage-0

RUN install_packages libssl3 ca-certificates
COPY --from=build /kube-watcher /kube-watcher

USER nobody

ENTRYPOINT ["/kube-watcher"]
