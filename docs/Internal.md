# Internal Documentation

> Maybe someone else can find these things useful too...

## How to kernel panic

Some people say you can do this:

```sh
echo 1 > /proc/sys/kernel/sysrq
echo c > /proc/sysrq-trigge
```

But it doesn't work on every distribution, for example ubuntu cloud images.

Instead, you can use QEMU ability to send NMI (non-maskable interrupt).

Enable panic on NMI, using SSH:

```sh
root@ubuntu:~# echo 1 > /proc/sys/kernel/panic_on_unrecovered_nmi
```

Send NMI through monitor:

```sh
./tools/monitor.sh nmi
```

You should see VM panic and reboot.

If you want VM to stop on panic instead, use `-no-reboot` option.
