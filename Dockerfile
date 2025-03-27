FROM rust:1.85.1-alpine AS builder
WORKDIR /usr/src/myapp
COPY app .
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static
RUN cargo install --path .

FROM alpine:3.21.3
COPY --from=builder /usr/local/cargo/bin/app /usr/local/bin/app
COPY ./app/static ./static
EXPOSE 3000

CMD ["app"]
