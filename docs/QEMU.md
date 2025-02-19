# QEMU configuration

> All scripts are executed from project top directory.

## Ubuntu Cloud Image

### Download

Download image from <https://cloud-images.ubuntu.com> in QCow2 format with KVM kernel. These images are small (~600M) and are optimized for VM usage.

For example:

> jammy-server-cloudimg-amd64-disk-kvm.img
>
> QCow2 UEFI/GPT Bootable disk image with linux-kvm KVM optimised kernel

Rename file to `disk.img` for convenience, and copy to project directory.

> Scripts use predefined filenames that can be changed by passing environment variables:
>
> `$ OS_IMAGE=path/to/disk.img ./launch.sh`

### Generate SSH keys

> If you are not familiar with SSH, read some documentation first.
>
> <https://www.openssh.com/manual.html>
>
> But basically, it allows user to execute commands on remote server (in our case server is running inside VM).
>
> Additionally, you can copy files from/to server using `scp`.

Execute:

```sh
./tools/gen-ssh.sh
```

This will generate 2 keys: private and public (`.pub` extension).

__Private__ key is for client (__host__) and __public__ key is for server (__guest VM__).

### Make cloud configuration image

In order to configure SSH and other stuff (e.g. users) one can use `cloud-init` utilities.

First, edit [config file](../tools/cloud-config.yml). Paste __public__ key into `ssh_authorized_keys:` field (make sure its a single line):

```yaml
    ssh_authorized_keys:
      - ssh-rsa AAAAB3N........D23SAA user@device
```

Install `cloud-utils` (Ubuntu / Fedora) or `cloud-image-utils` (Arch) in order to use `cloud-localds` command.

Execute:

```sh
./tools/make-seed.sh
```

> Documentation: <https://documentation.ubuntu.com/public-images/en/latest/public-images-how-to/use-local-cloud-init-ds/>

This will produce binary file `seed.img` with the configuration.

### First boot

> Cloud images come with 2 GB root file system which may fill up quickly. If image is resized (using `qemu-img`) __before__ booting with cloud-init seed image, the file system will be resized automatically. Otherwise, see the section below about resizing file system.

Install `qemu` packages (e.g. `qemu-system-x86_64`). Package names can vary in different distributions.

Execute:

```sh
./tools/launch-first-boot.sh
```

```log
...

         Starting Time & Date Service...
[  OK  ] Started Time & Date Service.
cloud-init[1055]: Cloud-init v. 24.4-0ubuntu1~22.04.1 running 'modules:config' at Tue, 18 Feb 2025 12:42:13 +0000. Up 11.35 seconds.

Ubuntu 22.04.5 LTS ubuntu ttyS0

...

<14>Feb 18 12:42:13 cloud-init: #############################################################
<14>Feb 18 12:42:13 cloud-init: -----BEGIN SSH HOST KEY FINGERPRINTS-----
<14>Feb 18 12:42:13 cloud-init: 256 SHA256:j6A+Fjh5c9dtSySgjHJ0K8NUAL1v7Z6S/TwEtVFJHdg root@ubuntu (ECDSA)
<14>Feb 18 12:42:13 cloud-init: 256 SHA256:9hPksPsKffQHey0dtyLgafqhA1rfKW8mjYBj1s12Pr4 root@ubuntu (ED25519)
<14>Feb 18 12:42:14 cloud-init: 3072 SHA256:j/ya2r/5FLdHLGnS9PH5mLc8mjMB3ErGgF92Syec1V0 root@ubuntu (RSA)
<14>Feb 18 12:42:14 cloud-init: -----END SSH HOST KEY FINGERPRINTS-----
<14>Feb 18 12:42:14 cloud-init: #############################################################

...
```

This should set up SSH and save changes to image.

Try to connect via SSH (in other terminal):

```sh
./tools/connect-ssh.sh
```

You should get welcoming text:

```text
Welcome to Ubuntu 22.04.5 LTS (GNU/Linux 5.15.0-1073-kvm x86_64)

 * Documentation:  https://help.ubuntu.com
 * Management:     https://landscape.canonical.com
 * Support:        https://ubuntu.com/pro

...
```

Try running some commands:

```sh
root@ubuntu:~# df
Filesystem     1K-blocks    Used Available Use% Mounted on
/dev/root        2051980 1483784    551812  73% /
tmpfs            1019472       0   1019472   0% /dev/shm
tmpfs             407792     544    407248   1% /run
tmpfs               5120       0      5120   0% /run/lock
/dev/vda15        106832    6190    100642   6% /boot/efi
tmpfs             203892       4    203888   1% /run/user/0
```

There are many ways to stop vm, but for now just do (__in SSH session!__)

```sh
root@ubuntu:~# shutdown now
```

> __Make sure to exit VM properly, if you don't want to lose data saved to disk__

### Making changes

QEMU supports a `snapshot` feature, where changes __are not__ saved to image. This is useful for fuzzing, but first we need to install packages/dependencies (and optionally, build a custom kernel).

There are 2 scripts for this:

```sh
./tools/launch-persistent.sh # for making changes to system
./tools/launch-snapshot.sh   # for fuzzing / experiments 
```

Execute:

```sh
./tools/launch-persistent.sh
```

You will be met with login message - connect via SSH instead (again):

```sh
./tools/connect-ssh.sh
```

> You can also execute single commands using `./tools/execute-ssh.sh CMD`

In order to compile C code, you need to install some packages:

```sh
root@ubuntu:~# apt-get update
root@ubuntu:~# apt install build-essential
```

This should install `g++`, `make` and other required packages.

> This can fill root file system completely, you might want to resize image at this point. See section below.

Now, shutdown the system:

```sh
root@ubuntu:~# shutdown now
```

*This should be enough to run black-box fuzzing, though you won't be able to detect memory bugs in Linux kernel.*

### Resizing image

First, resize image itself, 10G should be enough:

```sh
qemu-img resize disk.img 10G
```

Now, boot the VM:

> You might want to backup the image file, in case something goes wrong.

```sh
./tools/launch-persistent.sh
```

Before doing anything, determine device name:

```sh
root@ubuntu:~# fdisk -l
Disk /dev/vda: 2.2 GiB, 2361393152 bytes, 4612096 sectors
Units: sectors of 1 * 512 = 512 bytes
Sector size (logical/physical): 512 bytes / 512 bytes
I/O size (minimum/optimal): 512 bytes / 512 bytes
Disklabel type: gpt
Disk identifier: C7D1CFC4-329F-4776-BBFF-EFE0D4150C20

Device      Start     End Sectors  Size Type
/dev/vda1  227328 4612062 4384735  2.1G Linux filesystem
/dev/vda14   2048   10239    8192    4M BIOS boot
/dev/vda15  10240  227327  217088  106M EFI System
...
```

Root file system should be the largest one (`/dev/vda1`).

Grow partition 1 on `/dev/vda` using `growpart`:

```sh
root@ubuntu:~# growpart /dev/vda 1
CHANGED: partition=1 start=227328 old: size=4384735 end=4612063 new: size=20744159 end=20971487
```

And resize filesystem itself:

```sh
root@ubuntu:~# resize2fs /dev/vda1
resize2fs 1.46.5 (30-Dec-2021)
Filesystem at /dev/vda1 is mounted on /; on-line resizing required
old_desc_blocks = 1, new_desc_blocks = 2
The filesystem on /dev/vda1 is now 2593019 (4k) blocks long.
```

Verify:

```sh
root@ubuntu:~# df -h
Filesystem      Size  Used Avail Use% Mounted on
/dev/root       9.6G  1.9G  7.7G  20% /
```

---
>TODO copying files / testing environment
---
>TODO monitor
---
>TODO kernel
---
>TODO qmp
---
>TODO how to kernel panic
