FROM golang:1.27-bookworm@sha256:5cf287a799e6b94384bad13d16b14904c531f51ba65792237e122ce42b392f61 AS build
WORKDIR /build
ENV GOMAXPROCS=2
COPY scripts/ci/trusted-policy/ ./
RUN go mod download && go mod verify
RUN go test -mod=readonly ./... && CGO_ENABLED=0 go build -mod=readonly -trimpath -o /trusted-policy .
COPY .woodpecker.yaml /source/.woodpecker.yaml
COPY .gitleaks.toml /source/.gitleaks.toml
COPY scripts/ci/functional-run.sh scripts/ci/gitleaks-scan-tracked.sh /source/scripts/ci/
RUN /trusted-policy bundle /source > /config.json

FROM debian:bookworm-slim@sha256:99c0b25c46d0214d5a2091092e7908907dcaecd6a962cde0ac700acee222b6f3
COPY --from=build /trusted-policy /usr/local/bin/trusted-policy
COPY --from=build /config.json /policy/config.json
# The signing key is public; its readability must not depend on checkout umask.
COPY --chmod=0444 scripts/ci/trusted-policy/woodpecker-public.pem /policy/woodpecker-public.pem
USER 10001:10001
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/usr/local/bin/trusted-policy", "health"]
ENTRYPOINT ["/usr/local/bin/trusted-policy"]
