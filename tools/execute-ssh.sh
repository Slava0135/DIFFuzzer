#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

# Execute __single__ remote command using SSH
#
# ./execute-ssh.sh CMD

ssh -q -i "$KEY" -o "StrictHostKeyChecking no" -p "$SSH_PORT" root@localhost -t "$@"
