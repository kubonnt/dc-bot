FROM rust:latest
WORKDIR /app
RUN apt-get update  \
    && apt-get -y install cmake  \
    && apt-get -y install ffmpeg \
    && apt-get -y install libopus-dev \
    && apt-get -y install youtube-dl
COPY . /app
RUN cargo build --release
CMD ./target/release/dcbot
