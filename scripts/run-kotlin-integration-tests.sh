#!/bin/bash -eu

# The project should be mounted to this location
cd /app

# Trust the test CA certificate in the JVM keystore for SSL mock tests
keytool -importcert -file test-data/ssl-certs/ca-cert.pem \
    -keystore $JAVA_HOME/lib/security/cacerts \
    -storepass changeit -noprompt -alias wordpress-rs-test-ca

cd native/kotlin

./gradlew :api:kotlin:integrationTest
