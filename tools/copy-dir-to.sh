#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

# Copy directory from host __to remote__

scp -r -q -i "$KEY" -o "StrictHostKeyChecking no" -P "$SSH_PORT" "$1" "root@localhost:$2"
