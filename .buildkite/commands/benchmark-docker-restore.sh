#!/bin/bash

# TEMPORARY – measures how long restoring the Docker test site takes with different database settings.

set -u

# Give read/write permissions to `./` for all users
chmod -R a+rw ./

TIMEFORMAT='%R'

bench() {
	local label="$1" query="$2" samples=""
	for _ in 1 2 3 4 5; do
		local t
		t="$( { time curl -s -o /dev/null "http://localhost:4000/restore?$query"; } 2>&1 )"
		samples="$samples $t"
	done
	echo "BENCH $label:$samples"
}

variant() {
	local label="$1"

	echo "--- :stopwatch: $label"
	if ! make test-server > test-server.log 2>&1; then
		echo "BENCH $label: the test server failed to start"
		tail -n 40 test-server.log
		return 1
	fi

	# `make test-server` returns as soon as the backend is launched
	for _ in $(seq 1 60); do
		if curl -s -o /dev/null http://localhost:4000/; then break; fi
		sleep 2
	done

	docker inspect -f 'database container: cmd={{.Config.Cmd}} tmpfs={{.HostConfig.Tmpfs}}' "$(docker-compose ps -q database)"

	bench "$label | db only" "db=true&plugins=false"
	bench "$label | plugins only" "db=false&plugins=true"
	bench "$label | db + plugins" "db=true&plugins=true"
}

echo "--- :mag: Environment"
docker --version
docker-compose --version
docker info 2> /dev/null | grep -E 'Storage Driver|Backing Filesystem|Operating System|CPUs|Total Memory'
df -hT . /var/lib/docker 2> /dev/null

rm -f docker-compose.override.yml
variant "baseline"

cat > docker-compose.override.yml <<'YAML'
services:
    database:
        command: --debug-no-sync
YAML
variant "debug-no-sync"

cat > docker-compose.override.yml <<'YAML'
services:
    database:
        tmpfs:
            - /var/lib/mysql
YAML
variant "tmpfs datadir"

cat > docker-compose.override.yml <<'YAML'
services:
    database:
        command: --innodb-flush-log-at-trx-commit=0
YAML
variant "innodb-flush-log-at-trx-commit=0"

rm -f docker-compose.override.yml
