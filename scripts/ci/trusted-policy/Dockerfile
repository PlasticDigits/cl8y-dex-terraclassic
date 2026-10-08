FROM golang:1.26-bookworm@sha256:dc9ad6c05acc7a88e5b71bde60a5fe3bd4b9f0db209011711b464107438a8107 AS build
WORKDIR /build
ENV GOMAXPROCS=2
COPY scripts/ci/trusted-policy/ ./
RUN go mod download && go mod verify
RUN go test -mod=readonly ./... && CGO_ENABLED=0 go build -mod=readonly -trimpath -o /trusted-policy .
COPY .woodpecker.yaml /source/.woodpecker.yaml
COPY .gitleaks.toml /source/.gitleaks.toml
COPY scripts/ci/functional-run.sh scripts/ci/gitleaks-scan-tracked.sh /source/scripts/ci/
RUN /trusted-policy bundle /source > /config.json

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
COPY --from=build /trusted-policy /usr/local/bin/trusted-policy
COPY --from=build /config.json /policy/config.json
COPY scripts/ci/trusted-policy/woodpecker-public.pem /policy/woodpecker-public.pem
USER 10001:10001
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/usr/local/bin/trusted-policy", "health"]
ENTRYPOINT ["/usr/local/bin/trusted-policy"]
