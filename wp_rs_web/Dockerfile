FROM rust:1.88.0 AS builder

WORKDIR /app

COPY . .

RUN ls -la

# Set up NVM
ENV BASH_ENV=/root/.bash_env
RUN touch "${BASH_ENV}"
RUN echo '. "${BASH_ENV}"' >> ~/.bashrc

RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.2/install.sh | PROFILE="${BASH_ENV}" bash
RUN echo node > .nvmrc
RUN /bin/bash -c "nvm install"

# Build the CSS
RUN /bin/bash -c "npm install --prefix wp_rs_web"
RUN /bin/bash -c "npm run build:css --prefix wp_rs_web"

# Build the web binary
RUN cargo build -p wp_rs_web --bin wp_rs_web --release

#
# Build the runtime docker image
#

FROM debian:bookworm-slim

RUN apt update && apt install -y openssl && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app/wp_rs_web/assets
COPY --from=builder /app/target/release/wp_rs_web /app/wp_rs_web/wp_rs_web
COPY --from=builder /app/wp_rs_web/assets/style.css /app/wp_rs_web/assets/style.css
COPY --from=builder /app/wp_rs_web/templates /app/wp_rs_web/templates

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app/wp_rs_web

ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000

ENTRYPOINT ["/app/wp_rs_web/wp_rs_web"]
