# syntax=docker/dockerfile:1
FROM --platform=linux/amd64 rust:1.73

# Install system commands, Android SDK, and Ruby
RUN apt-get update  \
    && apt-get install -y coreutils git wget locales android-sdk android-sdk-build-tools \
    && apt-get install -y python3 \
    && apt-get -y autoclean

RUN ln -s /usr/bin/python3 /usr/bin/python & \
    ln -s /usr/bin/pip3 /usr/bin/pip

# Set up the default locale
RUN locale-gen en_US.UTF-8
ENV LANG="en_US.UTF-8" LANGUAGE="en_US:en"
ENV ANDROID_HOME=/usr/lib/android-sdk
ENV GRADLE_OPTS="-Xmx6G -XX:+HeapDumpOnOutOfMemoryError -Dorg.gradle.caching=true -Dorg.gradle.configureondemand=true -Dkotlin.compiler.execution.strategy=in-process -Dkotlin.incremental=false"

# Download the SDK Manager
RUN wget https://dl.google.com/android/repository/commandlinetools-linux-6858069_latest.zip \
	&& unzip commandlinetools-linux-6858069_latest.zip && rm commandlinetools-linux-6858069_latest.zip \
	&& mkdir /usr/lib/android-sdk/cmdline-tools \
	&& mv cmdline-tools /usr/lib/android-sdk/cmdline-tools/latest

ENV PATH="//usr/lib/android-sdk/cmdline-tools/latest/bin:${PATH}"

RUN yes | sdkmanager --licenses

# Uninstall '29.0.3' so that the builds won't complain about it being installed in incorrect location
RUN sdkmanager --uninstall "build-tools;29.0.3"

RUN sdkmanager --install \
  "build-tools;33.0.2" \
  "build-tools;34.0.0" \
  "platform-tools" \
  "platforms;android-33" \
  "platforms;android-34" \
  "ndk;25.1.8937393" \
  "ndk;25.2.9519653"

RUN rustup target add x86_64-linux-android \
  i686-linux-android \
  armv7-linux-androideabi \
  aarch64-linux-android
