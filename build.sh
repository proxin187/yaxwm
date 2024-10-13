#!/bin/sh

# Xephyr -br -ac -noreset -screen 800x600 :1

cargo build --release --bin yaxc

sudo cp target/release/yaxc /usr/bin/yaxc

DISPLAY=:2 cargo run --bin yaxum


