# Linux Kernel Documentation

> This document assumes you already have a working QEMU image.

## Useful links

- <https://davidaugustat.com/linux/how-to-compile-linux-kernel-on-ubuntu>
- <https://wiki.gentoo.org/wiki/Kernel/Configuration>
- <https://blog.cloudflare.com/a-gentle-introduction-to-linux-kernel-fuzzing/>

## Building kernel with KCov (Ubuntu)

### Download

Download source code from <https://www.kernel.org>.

To avoid problems, get the kernel version that is already being used in the system.

To check running kernel version, inside VM:

```sh
root@ubuntu:~# uname -r
5.15.0-1073-kvm
```

In this example (`Ubuntu 22.04.5`), you need to download kernel `5.15.x` (or you can install other supported kernel from official repositories).

### Setup VM

Make an image backup, just in case.

Start VM in __persistent__ mode, make sure it's using the correct image (path to image can be passed with `OS_IMAGE` variable):

> Kernel compilation is quite resource heavy, increase number of cores / available memory to speed up the process (4 cores + 8G should be enough).

```sh
./tools/launch-persistent.sh
```

> Make sure VM has enough disk on root file system (`df -h`). If not, resize image as described [in this document](./QEMU.md).

Copy kernel archive (path and filenames can be different):

```sh
./tools/copy-to.sh ~/Downloads/linux-5.15.178.tar.xz /linux-5.15.178.tar.xz
```

Connect to SSH:

```sh
./tools/connect-ssh.sh
```

Install required packages:

```sh
root@ubuntu:~# sudo apt install build-essential libncurses-dev bison flex libssl-dev libelf-dev fakeroot dwarves
```

Change working directory:

```sh
root@ubuntu:~# cd /
```

Extract archive:

```sh
root@ubuntu:/# tar -xvf linux-5.15.178.tar.xz
```

Change working directory to source directory:

```sh
root@ubuntu:/# cd linux-5.15.178/
```

### Configuration

> For more detailed setup, read [Syzkaller documentation](https://github.com/google/syzkaller/blob/464ac2eda061918b0834afc83052d755176d25a1/docs/linux/kernel_configs.md)

Copy configuration of the running kernel (__only loaded modules will be included__):

```sh
root@ubuntu:/linux-5.15.178# make localmodconfig
```

You can enable them manually, or, instead, you can include *all* installed modules (will increase compilation time):

```sh
root@ubuntu:/linux-5.15.178# cp -v /boot/config-$(uname -r) .config
'/boot/config-5.15.0-1073-kvm' -> '.config'
```

`./scripts/config` is used to change kernel configuration:

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -h
Manipulate options in a .config file from the command line.
Usage:
config options command ...
commands:
        --enable|-e option   Enable option
        --disable|-d option  Disable option
        --module|-m option   Turn option into a module
        --set-str option string
                             Set option to "string"
        --set-val option value
                             Set option to value
        --undefine|-u option Undefine option
        --state|-s option    Print state of option (n,y,m,undef)
```

Turn `brd` into module:

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -m BLK_DEV_RAM
```

It is used to run file systems inside RAM, for better speed and easy reset.

Enable KCov:

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -e KCOV -e KCOV_ENABLE_COMPARISONS
```

It's advised to disable full kernel instrumentation, to increase speed and make coverage more accurate:

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -d KCOV_INSTRUMENT_ALL
```

Instead, enable KCov on per-module basis (in each Makefile inside `fs` directory):

```sh
root@ubuntu:/linux-5.15.178# find fs -name Makefile | xargs -L1 -I {} bash -c 'echo "KCOV_INSTRUMENT := y" >> {}'
```

Disable `KASLR` for more deterministic behavior:

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -d RANDOMIZE_BASE
```

Don't forget to enable modules for file systems (search for section `File systems` inside `.config` file):

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -e XFS_FS -e BTRFS_FS -e F2FS_FS
```

Disable kernel signing, otherwise you will (likely) get build error:

> Issue: <https://askubuntu.com/questions/1329538/compiling-kernel-5-11-11-and-later>

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -d SYSTEM_TRUSTED_KEYS -d SYSTEM_REVOCATION_KEYS --set-str CONFIG_SYSTEM_TRUSTED_KEYS "" --set-str CONFIG_SYSTEM_REVOCATION_KEYS ""
```

#### Enabling KASAN

Kernel Address Sanitizer (KASAN) is a dynamic memory safety error detector designed to find out-of-bounds and use-after-free bugs.

> Documentation: <https://docs.kernel.org/dev-tools/kasan.html>

You should only enable it when fuzzing kernel file systems in order to detect memory bugs, because __it adds a significant performance overhead__.

```sh
root@ubuntu:/linux-5.15.178# ./scripts/config -e KASAN -e KASAN_INLINE -e KASAN_GENERIC
```

### Build

Build with 6 cores (jobs):

```sh
root@ubuntu:/linux-5.15.178# fakeroot make -j6
```

> You may be asked questions about further module configuration, just pick default answers.

Make sure `make` finished successfully:

```sh
...
Kernel: arch/x86/boot/bzImage is ready  (#1)
root@ubuntu:/linux-5.15.178# echo $?
0
```

Otherwise, run `make` again, to get error message:

```sh
root@ubuntu:/linux-5.15.178# make
```

If you get error `multiple target patterns. stop` in some Makefile, you can try disabling that module.

### Install

Install kernel modules:

```sh
root@ubuntu:/linux-5.15.178# make modules_install
```

Install kernel:

```sh
root@ubuntu:/linux-5.15.178# make install
```

On next boot, new kernel will be used.

### Direct boot

In order to pass kernel command line arguments (and enable panics on KASAN reports for instance) QEMU direct boot is required.

Put Linux source directory in an archive:

```sh
root@ubuntu:/# tar -cvf /linux.tar /linux-5.15.178
```

Copy archive from VM:

```sh
./tools/copy-from.sh /linux.tar ~/path/to/linux.tar
```

Extract:

```sh
cd ~/path/to/
tar -xvf linux.tar
```

Source should be located at `~/path/to/linux-5.15.178` and `bzImage` file, required for direct boot, should be at `~/path/to/linux-5.15.178/arch/x86/boot/bzImage`.

Update `config.toml` accordingly and use environmental variables when launching scripts with said path.
