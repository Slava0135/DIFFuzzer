#!/bin/bash

: "${MONITOR_SOCKET_PATH:="/tmp/diffuzzer-qemu-monitor.sock"}"

# Execute single human QEMU monitor command
# Documentation: https://qemu-project.gitlab.io/qemu/system/monitor.html

echo "$@" | netcat -NU "$MONITOR_SOCKET_PATH"
