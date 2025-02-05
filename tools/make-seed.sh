#!/bin/bash

: "${CONFIG:="./cloud-config.yml"}"
: "${SEED_IMAGE:="./seed.img"}"

cloud-localds "$SEED_IMAGE" "$CONFIG"
