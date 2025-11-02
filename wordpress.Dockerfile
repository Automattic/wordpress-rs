ARG WORDPRESS_VERSION="latest"

FROM public.ecr.aws/docker/library/wordpress:${WORDPRESS_VERSION}

RUN apt-get update  \
  && apt-get install -y wget gpg

# https://docs.aws.amazon.com/corretto/latest/corretto-21-ug/generic-linux-install.html
# To use the Corretto Apt repositories on Debian-based systems, such as Ubuntu, import the \
# Corretto public key and then add the repository to the system list by using the following commands:
RUN wget -O - https://apt.corretto.aws/corretto.key | gpg --dearmor -o /usr/share/keyrings/corretto-keyring.gpg && \
  echo "deb [signed-by=/usr/share/keyrings/corretto-keyring.gpg] https://apt.corretto.aws stable main" | tee /etc/apt/sources.list.d/corretto.list

RUN apt-get update  \
  && apt-get install -y java-21-amazon-corretto-jdk android-sdk wget default-mysql-client less libssl-dev jo jq \
  && apt-get -y autoclean

# Install wp-cli
RUN curl -L https://github.com/wp-cli/wp-cli/releases/download/v2.12.0/wp-cli-2.12.0.phar --output /usr/bin/wp
RUN chmod +x /usr/bin/wp

# Create wpcli working directory
RUN mkdir -p /var/www/.wp-cli
ENV PATH="/root/.cargo/bin:${PATH}"
RUN chown -R www-data:www-data /var/www/.wp-cli/

# Run this command as root user since that's what the Docker will use when we run --http=http://localhost commands
RUN wp --allow-root package install wp-cli/restful

# Setup Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN rustup target add x86_64-linux-android i686-linux-android armv7-linux-androideabi aarch64-linux-android

# Setup Kotlin & Android
ENV ANDROID_HOME=/usr/lib/android-sdk

RUN wget https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip \
	&& unzip commandlinetools-linux-11076708_latest.zip && rm commandlinetools-linux-11076708_latest.zip \
	&& mkdir /usr/lib/android-sdk/cmdline-tools \
	&& mv cmdline-tools /usr/lib/android-sdk/cmdline-tools/latest

ENV PATH="//usr/lib/android-sdk/cmdline-tools/latest/bin:${PATH}"

RUN yes | sdkmanager --licenses

RUN sdkmanager --install \
  "ndk;28.1.13356709"

# Cache Gradle 8.7
RUN mkdir gradle-cache-tmp \
        && cd gradle-cache-tmp \
        && wget https://services.gradle.org/distributions/gradle-8.14-all.zip \
        && unzip gradle-8.14-all.zip \
        && touch settings.gradle \
        && gradle-8.14/bin/gradle wrapper --gradle-version 8.14 --distribution-type all \
        && ./gradlew \
        && cd .. \
        && rm -rf ./gradle-cache-tmp

# Setup Swift
ENV PATH="/root/.local/share/swiftly/bin:$PATH"
COPY scripts/docker/install-swift.sh /tmp/install-swift.sh
RUN chmod +x /tmp/install-swift.sh && /tmp/install-swift.sh && rm /tmp/install-swift.sh
RUN swift --version
