#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

# Copy directory __from remote__ to host

scp -r -q -i "$KEY" -o "StrictHostKeyChecking no" -P "$SSH_PORT" "root@localhost:$1" "$2" 
