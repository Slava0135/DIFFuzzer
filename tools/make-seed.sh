#!/bin/bash

: "${CONFIG:="./cloud-config.yml"}"
: "${SEED_IMAGE:="./seed.img"}"

# Make cloud init (seed) image from configuration

cloud-localds "$SEED_IMAGE" "$CONFIG"
