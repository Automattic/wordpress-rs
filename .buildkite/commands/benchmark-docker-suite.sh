#!/bin/bash -eu

# TEMPORARY – times a Docker-based integration test suite with the test database on disk or in memory.
#
# Usage: benchmark-docker-suite.sh <rust|kotlin> <disk|tmpfs>

SUITE=$1
VARIANT=$2

if [ "$VARIANT" = "disk" ]; then
	# What trunk does: no `tmpfs` mount, so the data directory stays on the container's volume
	sed -i '/^        tmpfs:$/,+1d' docker-compose.yml
fi

echo "--- :mag: Database service"
docker-compose config | sed -n '/^  database:/,/^  wordpress:/p'
if docker-compose config | grep -q tmpfs; then actual=tmpfs; else actual=disk; fi
if [ "$actual" != "$VARIANT" ]; then
	echo "Expected the database to be on $VARIANT, but it's on $actual"
	exit 1
fi

# Give read/write permissions to `./` for all users
chmod -R a+rw ./

start=$(date +%s)

echo "--- :docker: Setting up Test Server"
make test-server
setup_done=$(date +%s)

# Build everything first, so that the timed run below does nothing but run the tests
echo "--- :hammer: Building the tests"
case "$SUITE" in
	rust)
		docker exec -i wordpress /bin/bash -c 'cd /app && cargo test -p wp_api_integration_tests -p wp_mobile_integration_tests --no-run'
		;;
	kotlin)
		docker exec -i -e REPOSILITE_MIRROR_ENABLED -e REPOSILITE_MIRROR_URL wordpress /bin/bash -c 'cd /app/native/kotlin && ./gradlew --init-script /app/scripts/reposilite-mirror.gradle.kts :api:kotlin:integrationTestClasses'
		;;
	*)
		echo "Unknown suite: $SUITE"
		exit 1
		;;
esac
build_done=$(date +%s)

echo "--- 🧪 Running the tests"
make "test-$SUITE-integration"
tests_done=$(date +%s)

result="setup=$((setup_done - start)) build=$((build_done - setup_done)) tests=$((tests_done - build_done)) total=$((tests_done - start))"
echo "+++ :stopwatch: Result"
echo "BENCH suite=$SUITE variant=$VARIANT job=${BUILDKITE_PARALLEL_JOB:-0} $result"
buildkite-agent meta-data set "bench-$SUITE-$VARIANT-${BUILDKITE_PARALLEL_JOB:-0}" "$result"
