#!/bin/bash

: "${OS_IMAGE:="./disk.img"}"
: "${MONITOR_PORT:="55555"}"
: "${SSH_PORT:="2222"}"
: "${QMP_SOCKET_PATH:="/tmp/qemu-monitor.sock"}"
: "${SEED_IMAGE:="./seed.img"}"

qemu-system-x86_64  \
  -machine accel=kvm,type=q35 \
  -cpu host \
  -smp cores=2 \
  -m 2G \
  -nographic \
  -enable-kvm \
  -monitor tcp::"$MONITOR_PORT",server,nowait \
  -qmp unix:"$QMP_SOCKET_PATH",server,nowait \
  -device virtio-net-pci,netdev=net0 \
  -netdev user,id=net0,hostfwd=tcp::"$SSH_PORT"-:22 \
  -drive if=virtio,format=qcow2,file="$OS_IMAGE" \
  -drive if=virtio,format=raw,file="$SEED_IMAGE" \
