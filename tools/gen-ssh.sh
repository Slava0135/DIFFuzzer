#!/bin/bash

: "${FILENAME:="./ssh.key"}"

ssh-keygen -t rsa -f "$FILENAME" -q -N ""
