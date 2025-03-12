# File systems

> This document assumes you have a VM image set up, as described in [QEMU docs](./QEMU.md).

## Kernel

DIFFuzzer can be used for fuzzing kernel systems.

## Ext4

Nothing special.

## Btrfs

Nothing special.

## F2FS

You might want to enable compression and other options when building kernel.

>TODO

## XFS

Nothing special.

## bcacheFS

bcacheFS was accepted to mainline kernel in version 6.7+.

However, you need to install `bcachefs-tools` in order to format devices.

It's __not recommended__ that you install `bcachefs-tools` from your package manager (at least for Ubuntu), because they are often outdated.

Instead, you can build manually.

> Documentation: <https://bcachefs.org/GettingStarted/>

Clone repository with required tag/version. For example, for Linux 6.14 you need to use version 1.20.0:

```sh
git clone --depth 2 --branch v1.20.0 https://evilpiepirate.org/git/bcachefs-tools.git
```

Follow build instructions in `INSTALL.md`:

Build dependencies:

- libaio
- libblkid
- libclang
- libkeyutils
- liblz4
- libsodium
- liburcu
- libuuid
- libzstd
- pkg-config
- valgrind
- zlib1g

In addition a recent Rust toolchain is required (rustc, cargo), either by using
[rustup](https://rustup.rs/) or make sure to use a distribution where a recent
enough rustc is available. Please check `rust-version` in `Cargo.toml` to see
the minimum supported Rust version (MSRV).

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path # install Rust
. "$HOME/.cargo/env" # add Rust to environment
```

Debian (Bullseye or later) and Ubuntu (20.04 or later): you can install these with

```sh
apt install -y pkg-config libaio-dev libblkid-dev libkeyutils-dev \
    liblz4-dev libsodium-dev liburcu-dev libzstd-dev \
    uuid-dev zlib1g-dev valgrind libudev-dev udev git build-essential \
    python3 python3-docutils libclang-dev debhelper dh-python
```

Starting from Debian Trixie and Ubuntu 23.10, you will additionally need:

```sh
apt install -y systemd-dev
```

Then, just `make && make install`

Verify:

```sh
modprobe brd

bcachefs format /dev/ram0
mkdir /mnt/bcachefs
mount -t bcachefs /dev/ram0 /mnt/bcachefs
ls /mnt/bcachefs # There should be lost+found directory
...
umount /mnt/bcachefs

rmmod brd
```

## FUSE

DIFFuzzer can be used for fuzzing FUSE systems.

### LittleFS

#### Set up VM (Ubuntu)

Clone repository:

```sh
git clone https://github.com/littlefs-project/littlefs-fuse
```

__Repository must be at `/root/littlefs-fuse`__

```sh
cd /root/littlefs-fuse
```

Install dependencies:

```sh
sudo apt-get install libfuse-dev
```

In order to collect coverage and run greybox fuzzing, you need to install `lcov`:

```sh
sudo sudo apt-get install lcov
```

Edit `Makefile`:

```makefile
...
override CFLAGS += -std=c99 -Wall -pedantic
override CFLAGS += -fsanitize=address,undefined -fno-sanitize-recover -fprofile-arcs -ftest-coverage # add this line to enable sanitizers and coverage.
...
```

Build with debug mode (no optimizations, required for optimal coverage collection):

```sh
make DEBUG=1
```

If running blackbox fuzzing, you can build a normal (release) binary for better speed.

Setup coverage:

> Documentation: <https://linux.die.net/man/1/lcov>

```sh
lcov --zerocounters --directory .
lcov --capture --initial --directory . --output-file /tmp/lcov.info
```

Verify:

```sh
modprobe brd
./lfs --format /dev/ram0

mkdir /mnt/lfs
./lfs /dev/ram0 /mnt/lfs

echo "Hello, World!" > /mnt/lfs/hello.txt
ls /mnt/lfs
cat /mnt/lfs/hello.txt

umount /mnt/lfs

rmmod brd
```

Capture coverage:

```sh
lcov --capture --directory . --output-file /tmp/lcov.info
cat /tmp/lcov.info
```
