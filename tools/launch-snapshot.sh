#!/bin/bash

: "${OS_IMAGE:="./disk.img"}"
: "${SSH_PORT:="2222"}"
: "${MONITOR_SOCKET_PATH:="/tmp/diffuzzer-qemu-monitor.sock"}"
: "${QMP_SOCKET_PATH:="/tmp/diffuzzer-qemu-qmp.sock"}"

# QEMU direct boot with custom kernel and command line arguments
# Required for fuzzing with KASAN
: "${DIRECT_BOOT:=false}"
# Path to bzImage file.
: "${KERNEL_IMAGE_PATH:=".../linux-x.xx/arch/x86/boot/bzImage"}"
# Disk partition where root filesystem is located.
: "${ROOT_DISK_PARTITION:="/dev/vda1"}"

# Launch VM in snapshot mode (no changes are saved to disk).
# Use for fuzzing to avoid any image modifications.

# Kernel command line arguments.
# Documentation: https://docs.kernel.org/admin-guide/kernel-parameters.html
cmd_args=(
  # Useful when the kernel crashes before the normal console is initialized.
  "earlyprintk=serial"
  # Always panic on oopses. Default is to just kill the process.
  "oops=panic"
  # Kernel behaviour on panic: delay <timeout>
  #   timeout > 0: seconds before rebooting
  #   timeout = 0: wait forever
  #   timeout < 0: reboot immediately
  "panic=1"
  # Accelerate the execution of some system calls.
  # Vsyscall is legacy ABI though, probably not worth using this option.
  "vsyscall=native"
  # Network interfaces are renamed to give them predictable names when possible.
  # It is enabled by default; specifying 0 disables it.
  "net.ifnames=0"
  # Root file system location.
  "root=$ROOT_DISK_PARTITION"
  # Send all boot messages, kernel logs, and critical warnings through the first serial port.
  "console=ttyS0"
  # Panic the kernel on KASAN report.
  "kasan.fault=panic"
  # panic() instead of WARN(). Useful to cause kdump on a WARN().
  "panic_on_warn=1"
)

qemu_args=(
  -machine "accel=kvm,type=q35"
  -cpu "host"
  -smp "cores=2"
  -m "4G"
  -nographic
  -enable-kvm
  -monitor "unix:$MONITOR_SOCKET_PATH,server,nowait"
  -qmp "unix:$QMP_SOCKET_PATH,server,nowait"
  -device "virtio-net-pci,netdev=net0"
  -netdev "user,id=net0,hostfwd=tcp::$SSH_PORT-:22"
  -drive "if=virtio,format=qcow2,file=$OS_IMAGE"
  -snapshot
)

if [[ $DIRECT_BOOT = true ]]; then
  qemu_args+=(-append "${cmd_args[*]}")
  qemu_args+=(-kernel "$KERNEL_IMAGE_PATH")
fi

exec qemu-system-x86_64 "${qemu_args[@]}"
