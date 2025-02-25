#!/bin/bash

: "${QMP_SOCKET_PATH:="/tmp/diffuzzer-qemu-qmp.sock"}"

# Connect to QEMU monitor using QMP
# Documentation: https://qemu-project.gitlab.io/qemu/interop/qemu-qmp-ref.html
# Send this text to begin session: { "execute": "qmp_capabilities" }

nc -U "$QMP_SOCKET_PATH"
