FROM public.ecr.aws/docker/library/wordpress:${WORDPRESS_VERSION:-latest}

RUN apt-get update  \
  && apt-get install -y openjdk-17-jdk-headless android-sdk wget default-mysql-client less libssl-dev \
  && apt-get -y autoclean

# Install wp-cli
RUN curl -L https://github.com/wp-cli/wp-cli/releases/download/v2.6.0/wp-cli-2.6.0.phar --output /usr/bin/wp
RUN chmod +x /usr/bin/wp

# Create wpcli working directory
RUN mkdir -p /var/www/.wp-cli
ENV PATH="/root/.cargo/bin:${PATH}"
RUN chown -R www-data:www-data /var/www/.wp-cli/

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
  "ndk;25.1.8937393"

# Cache Gradle 8.7
RUN mkdir gradle-cache-tmp \
        && cd gradle-cache-tmp \
        && wget https://services.gradle.org/distributions/gradle-8.7-bin.zip \
        && unzip gradle-8.7-bin.zip \
        && touch settings.gradle \
        && gradle-8.7/bin/gradle wrapper --gradle-version 8.7 --distribution-type all \
        && ./gradlew \
        && cd .. \
        && rm -rf ./gradle-cache-tmp
