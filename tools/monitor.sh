#!/bin/bash

: "${MONITOR_PORT:="55555"}"

echo "$@" | netcat -N localhost "$MONITOR_PORT"
