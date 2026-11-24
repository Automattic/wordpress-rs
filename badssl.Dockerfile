# A copy of badssl.com for the TLS tests, from `scripts/badssl-server.sh`.
FROM public.ecr.aws/docker/library/ubuntu:24.04

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git make nginx openssl \
  && rm -rf /var/lib/apt/lists/*

COPY scripts/badssl-server.sh /usr/local/bin/badssl-server.sh
ENV BADSSL_DIR=/badssl
RUN badssl-server.sh prepare && mkdir /badssl-root

# Hands the root certificate to the containers that trust it, through the `badssl-root` volume.
CMD cp "$(badssl-server.sh root-cert)" /badssl-root/ && exec badssl-server.sh serve
