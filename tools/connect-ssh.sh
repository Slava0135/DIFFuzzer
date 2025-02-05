#!/bin/bash

: "${KEY:="./ssh.key"}"
: "${SSH_PORT:="2222"}"

ssh -q -i "$KEY" -o "StrictHostKeyChecking no" -p "$SSH_PORT" root@localhost
