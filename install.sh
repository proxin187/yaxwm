#!/bin/sh

cargo build --release --bin yaxc

sudo cp target/release/yaxc /usr/bin/yaxc

cargo build --release --bin yaxiwm

sudo cp target/release/yaxiwm /usr/bin/yaxiwm


