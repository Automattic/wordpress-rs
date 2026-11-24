#!/bin/bash -eu

# The project should be mounted to this location
cd /app/native/kotlin

# Trust the root of the badssl.com copy (the `badssl` service), so the TLS tests see the name
# mismatches it serves rather than an unknown root. OkHttp only reports a mismatch once it trusts
# the chain.
badssl_root=/badssl-root/ca-root.crt
for _ in $(seq 30); do
    [ -s "$badssl_root" ] && break
    sleep 1
done
if ! keytool -list -cacerts -storepass changeit -alias badssl-localhost-root >/dev/null 2>&1; then
    keytool -importcert -noprompt -cacerts -storepass changeit \
        -alias badssl-localhost-root -file "$badssl_root"
fi
export BADSSL_DOMAIN=badssl.localhost

./gradlew --init-script /app/scripts/reposilite-mirror.gradle.kts :api:kotlin:integrationTest
