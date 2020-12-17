# Create our build environment
FROM rust:1.48 as build

# Build dependencies first so they can be cached
RUN USER=root cargo new --bin --vcs none /four_in_a_row
WORKDIR /four_in_a_row
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Remove the default hello world program and copy our repository
RUN rm ./src/*.rs
RUN rm ./target/release/deps/four_in_a_row*
COPY ./src ./src
RUN cargo build --release

# Create our running environment
FROM debian:bullseye

WORKDIR /four_in_a_row

COPY --from=build /four_in_a_row/target/release/four_in_a_row ./four_in_a_row

CMD ["./four_in_a_row"]