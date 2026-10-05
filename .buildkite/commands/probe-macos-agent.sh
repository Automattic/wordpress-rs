#!/bin/bash

# TEMPORARY – measures where the time goes when the test site is restored between tests, while
# iterating on the native test server.

set -u

echo "--- :rust: Installing Rust"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
source "$HOME/.cargo/env"

echo "--- :homebrew: Installing Test Server Dependencies"
make test-server-native-deps > /dev/null 2>&1
make test-server-native || exit 1

STATE_DIR="$PWD/.wordpress"
MARIADB_PREFIX="$(brew --prefix)/opt/mariadb@11.4"
TIMEFORMAT='%R'

bench() {
	local label="$1" query="$2" samples=""
	for _ in 1 2 3 4 5; do
		local t
		t="$( { time curl -s -o /dev/null "http://127.0.0.1:4000/restore?$query"; } 2>&1 )"
		samples="$samples $t"
	done
	echo "BENCH $label:$samples"
}

restart_mariadb() {
	kill "$(cat "$STATE_DIR/run/mariadb.pid")"
	while [ -f "$STATE_DIR/run/mariadb.pid" ]; do sleep 1; done

	nohup "$MARIADB_PREFIX/bin/mariadbd" \
		--no-defaults \
		--basedir="$MARIADB_PREFIX" \
		--datadir="$STATE_DIR/mariadb" \
		--socket="$STATE_DIR/run/mariadb.sock" \
		--pid-file="$STATE_DIR/run/mariadb.pid" \
		--bind-address=127.0.0.1 \
		--port=3306 \
		"$@" > "$STATE_DIR/logs/mariadb-bench.log" 2>&1 &

	for _ in $(seq 1 20); do
		if "$MARIADB_PREFIX/bin/mariadb-admin" --no-defaults --socket="$STATE_DIR/run/mariadb.sock" --user=root ping > /dev/null 2>&1; then
			return 0
		fi
		sleep 1
	done
	echo "BENCH mariadbd did not start with: $*"
	tail -n 5 "$STATE_DIR/logs/mariadb-bench.log"
	return 1
}

echo "--- :stopwatch: Restore timings (seconds, 5 samples each)"
ls -la "$STATE_DIR/site/wp-content/dump.sql"
echo "plugin files: $(find "$STATE_DIR/site/wp-content/plugins-backup" -type f | wc -l)"
grep -c 'CREATE TABLE' "$STATE_DIR/site/wp-content/dump.sql"

bench "plugins only" "db=false&plugins=true"
bench "db only, as shipped (flush-log-at-trx-commit=0)" "db=true&plugins=false"

restart_mariadb && bench "db only, MariaDB defaults" "db=true&plugins=false"
restart_mariadb --innodb-flush-log-at-trx-commit=0 --debug-no-sync && bench "db only, + debug-no-sync" "db=true&plugins=false"
restart_mariadb --innodb-flush-log-at-trx-commit=0 --innodb-flush-method=nosync && bench "db only, + innodb-flush-method=nosync" "db=true&plugins=false"
restart_mariadb --innodb-flush-log-at-trx-commit=0 --innodb-flush-method=nosync --debug-no-sync --skip-innodb-doublewrite && bench "db only, + nosync + debug-no-sync + no doublewrite" "db=true&plugins=false"

echo "--- :stopwatch: wp-cli invocation"
for _ in 1 2 3; do
	( time ./scripts/native-test-server.sh exec wp --allow-root --http=http://localhost --path="$STATE_DIR/site" option get permalink_structure ) 2>&1 | tr '\n' ' '
	echo
done
