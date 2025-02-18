#!/bin/bash

: "${FILENAME:="./ssh.key"}"

# Generate SSH keys

ssh-keygen -t rsa -f "$FILENAME" -q -N ""
