#!/bin/bash

: "${QMP_SOCKET_PATH:="/tmp/qemu-monitor.sock"}"

echo Documentation: https://qemu-project.gitlab.io/qemu/interop/qemu-qmp-ref.html
echo Send to start: "{ \"execute\": \"qmp_capabilities\" }"
echo ""

nc -U "$QMP_SOCKET_PATH"
