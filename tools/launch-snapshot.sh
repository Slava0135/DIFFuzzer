#!/bin/bash

: "${OS_IMAGE:="./disk.img"}"
: "${SSH_PORT:="2222"}"
: "${MONITOR_SOCKET_PATH:="/tmp/diffuzzer-qemu-monitor.sock"}"
: "${QMP_SOCKET_PATH:="/tmp/diffuzzer-qemu-qmp.sock"}"

# Launch VM in snapshot mode (no changes are saved to disk).
# Use for fuzzing to avoid any image modifications.

qemu-system-x86_64  \
  -machine accel=kvm,type=q35 \
  -cpu host \
  -smp cores=2 \
  -m 2G \
  -nographic \
  -enable-kvm \
  -monitor unix:"$MONITOR_SOCKET_PATH",server,nowait \
  -qmp unix:"$QMP_SOCKET_PATH",server,nowait \
  -device virtio-net-pci,netdev=net0 \
  -netdev user,id=net0,hostfwd=tcp::"$SSH_PORT"-:22 \
  -drive if=virtio,format=qcow2,file="$OS_IMAGE" \
  -snapshot
