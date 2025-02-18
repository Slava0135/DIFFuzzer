#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

# Connect to VM using SSH (remote console)

ssh -q -i "$KEY" -o "StrictHostKeyChecking no" -p "$SSH_PORT" root@localhost
