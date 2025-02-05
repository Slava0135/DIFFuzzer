#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

scp -q -i "$KEY" -o "StrictHostKeyChecking no" -P "$SSH_PORT" "root@localhost:$1" "$2" 
