#!/bin/bash

cargo build --release
sudo mv target/release/seva /usr/bin/seva
