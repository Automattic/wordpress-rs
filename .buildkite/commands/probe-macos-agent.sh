#!/bin/bash

# TEMPORARY – prints what the macOS CI VM provides, while iterating on the native test server.

echo "--- :mag: System"
sw_vers
uname -a
echo "cpus: $(sysctl -n hw.ncpu), memory: $(( $(sysctl -n hw.memsize) / 1024 / 1024 / 1024 ))GB"
df -h /
echo "user: $(id)"
echo "pwd: $(pwd)"
echo "shell: $SHELL, bash: $BASH_VERSION"
echo "open files limit: $(ulimit -n)"
if sudo -n true 2> /dev/null; then echo "passwordless sudo: yes"; else echo "passwordless sudo: no"; fi

echo "--- :mag: Tools"
for tool in brew php wp mariadb mysql httpd jo jq java gradle cargo rustup node docker nc unzip sdkmanager adb; do
	printf '%-12s %s\n' "$tool" "$(command -v "$tool" || echo '<missing>')"
done
/usr/sbin/httpd -v
echo "PATH=$PATH"

echo "--- :mag: Java + Android"
/usr/libexec/java_home -V 2>&1
echo "JAVA_HOME=${JAVA_HOME:-<unset>}"
echo "ANDROID_HOME=${ANDROID_HOME:-<unset>}"
echo "ANDROID_SDK_ROOT=${ANDROID_SDK_ROOT:-<unset>}"
ls -la "$HOME/Library/Android/sdk" 2>&1
ls -la "$HOME/.gradle" 2>&1 | head -20

echo "--- :mag: Homebrew"
brew --version
brew --prefix
brew config
echo "Formulae: $(brew list --formula | tr '\n' ' ')"
echo "Casks: $(brew list --cask | tr '\n' ' ')"
ls "$(brew --prefix)/etc/httpd" 2>&1 | head

echo "--- :mag: Network"
lsof -nP -iTCP -sTCP:LISTEN
grep -v '^#' /etc/hosts

echo "--- :mag: Environment variable names"
env | cut -d= -f1 | sort | tr '\n' ' '
echo

echo "--- :mag: Gradle configuration without an Android SDK"
brew install openjdk@21
JAVA_HOME="$(brew --prefix openjdk@21)/libexec/openjdk.jdk/Contents/Home"
export JAVA_HOME
echo "GRADLE_OPTS is ${#GRADLE_OPTS} characters long"
cd native/kotlin || exit 1
./gradlew :api:kotlin:help
echo "PROBE RESULT plain: exit=$?"
./gradlew --configure-on-demand :api:kotlin:help
echo "PROBE RESULT configure-on-demand: exit=$?"
