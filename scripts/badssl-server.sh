#!/bin/bash
#
# Runs a copy of badssl.com (https://github.com/chromium/badssl.com) on this machine, so the
# tests of how TLS failures are reported don't depend on the live site. It serves the same
# sites under `badssl.localhost`, with certificates signed by a root it generates. Nothing
# trusts that root unless it's added to a trust store.
#
#   prepare      Fetch badssl.com, generate its certificates and write its nginx config.
#   serve        Run nginx in the foreground (for Docker).
#   start        Run nginx in the background.
#   stop         Stop the nginx `start` ran.
#   root-cert    Print the path of the root certificate that signs every site's certificate.
#   refused-handshakes
#                Print how many TLS handshakes clients have refused, such as for an unknown root.
#
# `BADSSL_DIR` sets where it works (default `.badssl` in the current directory). Tests find the
# sites through `BADSSL_DOMAIN=badssl.localhost`.
#
# macOS resolves any `*.localhost` name to this machine, so no /etc/hosts change is needed, and
# lets any user listen on ports 80 and 443. In Docker, give the container a network alias for
# each site a test uses.

set -euo pipefail

# The badssl.com commit to run.
BADSSL_COMMIT=bfc80f7c2bf0873e2fdc9ba79f38a5afd93570fb
BADSSL_REPOSITORY=https://github.com/chromium/badssl.com
DOMAIN=badssl.localhost

BADSSL_DIR="${BADSSL_DIR:-$PWD/.badssl}"
mkdir -p "$BADSSL_DIR"
BADSSL_DIR=$(cd "$BADSSL_DIR" && pwd)
SOURCE_DIR="$BADSSL_DIR/source"
SITE_DIR="$BADSSL_DIR/site"
NGINX_CONF="$BADSSL_DIR/nginx.conf"

# badssl.com's certificates need OpenSSL 3: macOS's own `openssl` is LibreSSL.
if [ "$(uname)" == "Darwin" ] && command -v brew >/dev/null; then
    PATH="$(brew --prefix openssl@3)/bin:$PATH"
fi

fetch() {
    if [ "$(git -C "$SOURCE_DIR" rev-parse HEAD 2>/dev/null)" == "$BADSSL_COMMIT" ]; then
        return
    fi
    rm -rf "$SOURCE_DIR"
    git init -q "$SOURCE_DIR"
    git -C "$SOURCE_DIR" fetch -q --depth 1 "$BADSSL_REPOSITORY" "$BADSSL_COMMIT"
    git -C "$SOURCE_DIR" checkout -q FETCH_HEAD
}

# Writes what badssl.com's Jekyll build would, for the nginx config: it only uses the `site`
# variables replaced here. The other changes let current nginx load it on any machine:
# - Current nginx no longer has the `ssl` directive (Ubuntu 24.04's 1.24 still does), so TLS
#   moves to `listen … ssl`. Every port but 80 serves TLS.
# - The keys are read from the generated set, not from `/etc/keys`.
# - Each port is also served on IPv6 where the machine has it: macOS resolves `*.localhost` to
#   `::1` first.
render_site() {
    rm -rf "$SITE_DIR"
    cp -R "$SOURCE_DIR" "$SITE_DIR"
    rm -rf "$SITE_DIR/.git"

    local ipv6=""
    if [ "$(uname)" == "Darwin" ] || [ -s /proc/net/if_inet6 ]; then
        ipv6=1
    fi

    # shellcheck disable=SC2016 # The single-quoted text is Perl, which expands it itself.
    find "$SITE_DIR" -name '*.conf' -not -path "$SITE_DIR/certs/*" -print0 |
        SITE="$SITE_DIR" CERTS="$SITE_DIR/certs/sets/current/gen" DOMAIN="$DOMAIN" IPV6="$ipv6" \
            xargs -0 perl -i -pe '
                $_ = "" if $. <= 2 && /^---$/;    # the empty Jekyll front matter
                s/\{\{ site\.(serving-path|serving_folder) \}\}/$ENV{SITE}/g;
                s/\{\{ site\.domain \}\}/$ENV{DOMAIN}/g;
                s/\{\{ site\.cert-path \}\}/$ENV{CERTS}\/chain/g;
                s/\{\{ site\.dhparam-path \}\}/$ENV{CERTS}\/dhparam/g;
                s#/etc/keys/#$ENV{CERTS}/key/#g;
                $_ = "" if /^\s*ssl on;/;
                if (/^(\s*)listen\s+(\d+)\s*;(.*)$/) {
                    my $ssl = $2 == 80 ? "" : " ssl";
                    $_ = "$1listen $2$ssl;$3\n";
                    $_ .= "$1listen [::]:$2$ssl;\n" if $ENV{IPV6};
                }
                close ARGV if eof;    # so `$.` counts each file from 1
            '
}

prepare() {
    fetch
    # What badssl.com's `make certs-test` does, for our domain rather than `badssl.test`.
    (cd "$SOURCE_DIR/certs" && make test O=sets/test D="$DOMAIN" >"$BADSSL_DIR/certs.log" 2>&1) ||
        { tail -20 "$BADSSL_DIR/certs.log"; exit 1; }
    rm -rf "$SOURCE_DIR/certs/sets/current"
    cp -R "$SOURCE_DIR/certs/sets/test" "$SOURCE_DIR/certs/sets/current"
    render_site

    cat >"$NGINX_CONF" <<EOF
worker_processes 1;
pid $BADSSL_DIR/nginx.pid;
# Logging at info records each TLS handshake a client refused, which shows the tests reached it.
error_log $BADSSL_DIR/error.log info;
events {}
http {
    access_log $BADSSL_DIR/access.log;
    include $SITE_DIR/nginx.conf;
}
EOF
    nginx -t -q -c "$NGINX_CONF" -p "$BADSSL_DIR"
}

case "${1:-}" in
    prepare) prepare ;;
    serve) exec nginx -c "$NGINX_CONF" -p "$BADSSL_DIR" -g 'daemon off;' ;;
    start) nginx -c "$NGINX_CONF" -p "$BADSSL_DIR" ;;
    stop) nginx -c "$NGINX_CONF" -p "$BADSSL_DIR" -s stop ;;
    root-cert) echo "$SITE_DIR/certs/sets/current/gen/crt/ca-root.crt" ;;
    refused-handshakes)
        # A client that refuses the certificate either sends an alert or just closes the connection.
        cat "$BADSSL_DIR/error.log" 2>/dev/null |
            grep -cE 'SSL_do_handshake\(\) failed|peer closed connection in SSL handshake' || true
        ;;
    *)
        echo "Usage: $0 prepare|serve|start|stop|root-cert|refused-handshakes" >&2
        exit 1
        ;;
esac
