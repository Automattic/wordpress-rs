FROM public.ecr.aws/docker/library/wordpress:${WORDPRESS_VERSION:-latest}

RUN apt-get update  \
  && apt-get install -y openjdk-17-jdk-headless android-sdk wget \
  && apt-get -y autoclean

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
