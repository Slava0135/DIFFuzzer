# File systems

> This document assumes you have a VM image set up, as described in [QEMU docs](./QEMU.md).

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
