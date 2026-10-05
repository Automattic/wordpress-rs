#!/bin/bash

# Runs the integration test site directly on this Mac instead of in Docker, for machines that can't
# run Docker – our macOS CI jobs run in VMs, and the hardware doesn't support nested virtualization.
#
# This reproduces what `docker-compose.yml` and `wordpress.Dockerfile` set up: WordPress served by
# Apache + mod_php on http://localhost, a MariaDB server, and the integration tests backend on port
# 4000. The site itself is set up by the same `setup-test-site.sh` that the Docker image uses.
#
# Everything this script creates lives in `.wordpress/` at the root of the repository.
#
# Usage: scripts/native-test-server.sh <command>
#
#   install-deps    Install PHP, MariaDB, Apache and jo using Homebrew
#   start           Create a fresh test site, then start the servers
#   stop            Stop the servers
#   exec <command>  Run a command in the environment the integration tests need

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Keep these in lockstep with `docker-compose.yml` and `wordpress.Dockerfile`, so that the tests run
# against the same software in both environments.
WORDPRESS_VERSION="${WORDPRESS_VERSION:-6.8.1}"
# The `wordpress:6.8.1` image is built on `php:8.2-apache`.
PHP_FORMULA="php@8.2"
# Homebrew only carries LTS releases of MariaDB – this is the closest one to the `mariadb:11.2` image.
MARIADB_FORMULA="mariadb@11.4"
WP_CLI_VERSION="2.12.0"
WP_CLI_RESTFUL_VERSION="0.4.1"

DB_PORT="${WP_TEST_DB_PORT:-3306}"
BACKEND_PORT=4000

STATE_DIR="$REPO_ROOT/.wordpress"
SITE_DIR="$STATE_DIR/site"
DB_DIR="$STATE_DIR/mariadb"
RUN_DIR="$STATE_DIR/run"
LOG_DIR="$STATE_DIR/logs"
BIN_DIR="$STATE_DIR/bin"
WP_CLI_DIR="$STATE_DIR/wp-cli"
WP_CLI_PHAR="$WP_CLI_DIR/wp-cli-$WP_CLI_VERSION.phar"
HTTPD_CONF="$STATE_DIR/httpd.conf"
PHP_INI="$STATE_DIR/php.ini"
DB_SOCKET="$RUN_DIR/mariadb.sock"

if ! command -v brew > /dev/null; then
	echo "Homebrew is required to run the test server without Docker – see https://brew.sh" >&2
	exit 1
fi

BREW_PREFIX="$(brew --prefix)"
PHP_PREFIX="$BREW_PREFIX/opt/$PHP_FORMULA"
MARIADB_PREFIX="$BREW_PREFIX/opt/$MARIADB_FORMULA"
HTTPD_PREFIX="$BREW_PREFIX/opt/httpd"

# The environment that `wp_cli` – and so the integration tests and their backend – needs to find
# the site: our `wp` wrapper and the `mariadb` client on the PATH, and the location of the site.
export PATH="$BIN_DIR:$MARIADB_PREFIX/bin:$PATH"
export WP_TEST_SITE_PATH="$SITE_DIR"
export WP_TEST_DB_HOST="127.0.0.1"
export WP_TEST_DB_PORT="$DB_PORT"

install_deps() {
	brew install "$PHP_FORMULA" "$MARIADB_FORMULA" httpd jo
}

require_deps() {
	local missing=()

	[ -x "$PHP_PREFIX/bin/php" ] || missing+=("$PHP_FORMULA")
	[ -x "$MARIADB_PREFIX/bin/mariadbd" ] || missing+=("$MARIADB_FORMULA")
	[ -x "$HTTPD_PREFIX/bin/httpd" ] || missing+=("httpd")
	command -v jo > /dev/null || missing+=("jo")

	if [ ${#missing[@]} -gt 0 ]; then
		echo "Missing dependencies: ${missing[*]}" >&2
		echo "Install them with \`make test-server-native-deps\`" >&2
		exit 1
	fi
}

# Fail early if something else – most likely the Docker test server – is using one of our ports.
require_free_port() {
	local port="$1"

	if nc -z 127.0.0.1 "$port" > /dev/null 2>&1; then
		echo "Port $port is already in use. If the Docker test server is running, stop it with \`make stop-server\`." >&2
		exit 1
	fi
}

# Runs the given command until it succeeds, giving up after 30 seconds.
wait_for() {
	local description="$1"
	shift

	local tries=0
	until "$@" > /dev/null 2>&1; do
		tries=$((tries + 1))
		if [ "$tries" -gt 30 ]; then
			echo "Timed out waiting for $description. Logs are in $LOG_DIR:" >&2
			tail -n 50 "$LOG_DIR"/*.log >&2 || true
			exit 1
		fi
		sleep 1
	done
}

stop_process() {
	local pid_file="$1"
	local name="$2"

	[ -f "$pid_file" ] || return 0

	local pid
	pid="$(cat "$pid_file")"

	# A PID file that outlived its process (after a reboot, say) may now name an unrelated one.
	case "$(ps -p "$pid" -o comm= 2> /dev/null)" in
		*"$name"*)
			kill "$pid"
			while kill -0 "$pid" 2> /dev/null; do
				sleep 1
			done
			;;
	esac

	rm -f "$pid_file"
}

stop() {
	stop_process "$RUN_DIR/backend.pid" wp_api_integration_tests_backend
	stop_process "$RUN_DIR/httpd.pid" httpd
	stop_process "$RUN_DIR/mariadb.pid" mariadbd
}

# Homebrew's `php.ini` is PHP's development configuration, which reports every deprecation and
# notice. Replace it with what the `wordpress` Docker image runs: PHP's built-in defaults, plus the
# settings that the image adds to them.
configure_php() {
	cat > "$PHP_INI" <<-'INI'
		error_reporting = E_ERROR | E_WARNING | E_PARSE | E_CORE_ERROR | E_CORE_WARNING | E_COMPILE_ERROR | E_COMPILE_WARNING | E_RECOVERABLE_ERROR
		display_errors = Off
		display_startup_errors = Off
		log_errors = On
		error_log = /dev/stderr
		log_errors_max_len = 1024
		ignore_repeated_errors = On
		ignore_repeated_source = Off
		html_errors = Off

		opcache.memory_consumption = 128
		opcache.interned_strings_buffer = 8
		opcache.max_accelerated_files = 4000
		opcache.revalidate_freq = 2
	INI
}

install_wp_cli() {
	mkdir -p "$BIN_DIR" "$WP_CLI_DIR"

	if [ ! -f "$WP_CLI_PHAR" ]; then
		curl -fsSL "https://github.com/wp-cli/wp-cli/releases/download/v$WP_CLI_VERSION/wp-cli-$WP_CLI_VERSION.phar" -o "$WP_CLI_PHAR"
	fi

	# `wp_cli` and `setup-test-site.sh` both run plain `wp`, so put a wrapper on the PATH that runs the
	# pinned WP-CLI with the pinned PHP, without picking up any WP-CLI configuration from this machine.
	#
	# The Docker image never needs to unpack WordPress, which takes WP-CLI more than PHP's default
	# 128MB of memory – this is the limit that the `wordpress:cli` image sets for the same reason.
	cat > "$BIN_DIR/wp" <<-EOF
		#!/bin/bash
		export PATH="$MARIADB_PREFIX/bin:\$PATH"
		export WP_CLI_PACKAGES_DIR="$WP_CLI_DIR/packages"
		export WP_CLI_CACHE_DIR="$WP_CLI_DIR/cache"
		export WP_CLI_CONFIG_PATH="$WP_CLI_DIR/config.yml"
		exec "$PHP_PREFIX/bin/php" -c "$PHP_INI" -d memory_limit=512M "$WP_CLI_PHAR" "\$@"
	EOF
	chmod +x "$BIN_DIR/wp"

	# `wp_cli` passes `--http` to every command, which WP-CLI rejects unless this package is installed.
	local installed_packages
	installed_packages="$(wp package list --fields=name --format=csv)"
	if ! grep -q '^wp-cli/restful$' <<< "$installed_packages"; then
		wp package install "wp-cli/restful:$WP_CLI_RESTFUL_VERSION"
	fi
}

start_database() {
	rm -rf "$DB_DIR"

	"$MARIADB_PREFIX/bin/mariadb-install-db" \
		--no-defaults \
		--basedir="$MARIADB_PREFIX" \
		--datadir="$DB_DIR" \
		--auth-root-authentication-method=normal \
		--skip-test-db \
		> "$LOG_DIR/mariadb-install.log" 2>&1 || {
		cat "$LOG_DIR/mariadb-install.log" >&2
		exit 1
	}

	# The database is recreated on every start and restored from a dump after most tests, so there's
	# nothing worth an `fsync` (which is particularly slow on macOS) on every commit.
	nohup "$MARIADB_PREFIX/bin/mariadbd" \
		--no-defaults \
		--basedir="$MARIADB_PREFIX" \
		--datadir="$DB_DIR" \
		--socket="$DB_SOCKET" \
		--pid-file="$RUN_DIR/mariadb.pid" \
		--bind-address=127.0.0.1 \
		--port="$DB_PORT" \
		--innodb-flush-log-at-trx-commit=0 \
		> "$LOG_DIR/mariadb.log" 2>&1 &

	wait_for "MariaDB" "$MARIADB_PREFIX/bin/mariadb-admin" --no-defaults --socket="$DB_SOCKET" --user=root ping

	# The same database and credentials as the `database` service in `docker-compose.yml`
	"$MARIADB_PREFIX/bin/mariadb" --no-defaults --socket="$DB_SOCKET" --user=root <<-SQL
		CREATE DATABASE wordpress;
		CREATE USER 'wordpress'@'localhost' IDENTIFIED BY 'wordpress';
		CREATE USER 'wordpress'@'%' IDENTIFIED BY 'wordpress';
		GRANT ALL PRIVILEGES ON wordpress.* TO 'wordpress'@'localhost';
		GRANT ALL PRIVILEGES ON wordpress.* TO 'wordpress'@'%';
	SQL
}

install_wordpress() {
	rm -rf "$SITE_DIR"

	wp core download --path="$SITE_DIR" --version="$WORDPRESS_VERSION"

	# The same extra configuration as `WORDPRESS_CONFIG_EXTRA` in `docker-compose.yml`
	wp config create \
		--path="$SITE_DIR" \
		--dbname=wordpress \
		--dbuser=wordpress \
		--dbpass=wordpress \
		--dbhost="127.0.0.1:$DB_PORT" \
		--skip-check \
		--extra-php <<-'PHP'
			# Allow application passwords to be used without HTTPS
			define( 'WP_ENVIRONMENT_TYPE', 'local' );

			# Disable auto-update – it makes the tests super unstable
			define( 'WP_AUTO_UPDATE_CORE', false );
		PHP

	# The same `.htaccess` that the `wordpress` Docker image ships with
	cat > "$SITE_DIR/.htaccess" <<-'HTACCESS'
		# BEGIN WordPress

		RewriteEngine On
		RewriteRule .* - [E=HTTP_AUTHORIZATION:%{HTTP:Authorization}]
		RewriteBase /
		RewriteRule ^index\.php$ - [L]
		RewriteCond %{REQUEST_FILENAME} !-f
		RewriteCond %{REQUEST_FILENAME} !-d
		RewriteRule . /index.php [L]

		# END WordPress
	HTACCESS
}

start_web_server() {
	local modules_dir="$HTTPD_PREFIX/lib/httpd/modules"

	# Everything listens on port 80 because the tests expect the site at `http://localhost`. macOS
	# lets unprivileged processes bind to a low port, but only on all interfaces – hence `Listen 80`
	# rather than `Listen 127.0.0.1:80`.
	cat > "$HTTPD_CONF" <<-EOF
		ServerRoot "$STATE_DIR"
		ServerName localhost
		Listen 80

		DefaultRuntimeDir "$RUN_DIR"
		PidFile "$RUN_DIR/httpd.pid"
		ErrorLog "$LOG_DIR/httpd-error.log"
		LogFormat "%h %l %u %t \"%r\" %>s %b %D" common
		CustomLog "$LOG_DIR/httpd-access.log" common

		LoadModule mpm_prefork_module "$modules_dir/mod_mpm_prefork.so"
		LoadModule unixd_module "$modules_dir/mod_unixd.so"
		LoadModule authz_core_module "$modules_dir/mod_authz_core.so"
		LoadModule log_config_module "$modules_dir/mod_log_config.so"
		LoadModule mime_module "$modules_dir/mod_mime.so"
		LoadModule dir_module "$modules_dir/mod_dir.so"
		LoadModule rewrite_module "$modules_dir/mod_rewrite.so"
		LoadModule php_module "$PHP_PREFIX/lib/httpd/modules/libphp.so"
		PHPIniDir "$PHP_INI"

		TypesConfig "$BREW_PREFIX/etc/httpd/mime.types"
		DirectoryIndex index.php index.html
		DocumentRoot "$SITE_DIR"

		<Directory "$SITE_DIR">
			Options FollowSymLinks
			AllowOverride All
			Require all granted
		</Directory>

		<FilesMatch \.php$>
			SetHandler application/x-httpd-php
		</FilesMatch>
	EOF

	"$HTTPD_PREFIX/bin/httpd" -f "$HTTPD_CONF" -k start

	wait_for "Apache" curl --silent --output /dev/null http://localhost/
}

start_backend() {
	cargo build --quiet --release -p wp_api_integration_tests_backend

	ROCKET_PORT="$BACKEND_PORT" nohup ./target/release/wp_api_integration_tests_backend \
		> "$LOG_DIR/wp_api_integration_tests_backend.log" 2>&1 &
	echo $! > "$RUN_DIR/backend.pid"

	wait_for "the integration tests backend" curl --silent --output /dev/null "http://127.0.0.1:$BACKEND_PORT/"
}

start() {
	require_deps

	stop
	require_free_port 80
	require_free_port "$DB_PORT"
	require_free_port "$BACKEND_PORT"

	mkdir -p "$RUN_DIR" "$LOG_DIR"
	rm -f "$LOG_DIR"/*.log

	echo "--- :wordpress: Installing WordPress $WORDPRESS_VERSION"
	configure_php
	install_wp_cli
	start_database
	install_wordpress
	start_web_server

	(cd "$SITE_DIR" && WORDPRESS_RS_REPO_ROOT="$REPO_ROOT" bash "$REPO_ROOT/scripts/setup-test-site.sh")

	echo "--- :rust: Starting the integration tests backend"
	start_backend
}

cd "$REPO_ROOT"

case "${1:-}" in
	install-deps)
		install_deps
		;;
	start)
		start
		;;
	stop)
		stop
		;;
	exec)
		shift
		exec "$@"
		;;
	*)
		sed -n '/^# Usage:/,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
		exit 1
		;;
esac
