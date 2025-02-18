#!/bin/bash

: "${MONITOR_PORT:="55555"}"

# Execute single human QEMU monitor command
# Documentation: https://qemu-project.gitlab.io/qemu/system/monitor.html

echo "$@" | netcat -N localhost "$MONITOR_PORT"
