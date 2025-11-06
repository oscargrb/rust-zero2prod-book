FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef

WORKDIR /app

RUN apt update && apt install lld clang -y

FROM chef AS planner

COPY . .

#Computea lock-likefileforourproject
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

#Build our projectdependencies,not ourapplication!
RUN cargo chef cook --release --recipe-path recipe.json

#Upto thispoint,ifourdependency treestaysthesame,
#alllayersshouldbecached.

COPY . .

ENV SQLX_OFFLINE=true

#Build our project
RUN cargo build --release --bin zero2prod

FROM debian:bullseye-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    #Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod zero2prod

COPY configuration configuration

ENV APP_ENVIRONMENT=production

ENTRYPOINT ["./zero2prod"]