#!/bin/sh

# Xephyr -br -ac -noreset -screen 800x600 :1

cargo build --release --bin yaxc

sudo cp target/release/yaxc /usr/bin/yaxc

DISPLAY=:1 cargo run --bin yaxiwm


