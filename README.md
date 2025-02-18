# DIFFuzzer - Differential Filesystem Fuzzer

> IMPORTANT: Project is Work in Progress (WIP).

__DIFFuzzer__ - is a fuzzer, that aims to find __memory__ and __semantic__ bugs in __kernel__ (Linux) and __userspace__ (FUSE) filesystems.

It expands on previous works, such as:

- [Hydra](https://dl.acm.org/doi/abs/10.1145/3341301.3359662), filesystem fuzzing framework.
- [Dogfood](https://dl.acm.org/doi/abs/10.1145/3377811.3380350), filsystem test workload generator.
- [CrashMonkey](https://dl.acm.org/doi/abs/10.1145/3320275), filesystem crash consistency testing framework.
- [Metis](https://www.usenix.org/conference/fast24/presentation/liu-yifei), filesystem model checking tool.
- [SibyLFS](https://dl.acm.org/doi/abs/10.1145/2815400.2815411), oracle-based testing for filesystems.
- and other...

Key features:

- __Filesystem Semantics__ - to generate "good" inputs, filesystem semantics must be modelled properly (as was shown in [Hydra](https://dl.acm.org/doi/abs/10.1145/3341301.3359662)).
- __Differential__ - two filesystems are tested against same input and differences in their execution are observed in order to discover __semantic__ bugs.
- __Coverage Guided__ - similar to [Syzkaller](https://github.com/google/syzkaller), kernel coverage (KCov) is used to pick and mutate "interesting" inputs.
- __Native and QEMU__ - can be run on local machine as well as in VM using __QEMU__.
- __Easy Filesystem Integration__ - see: [Adding New Filesystem](#adding-new-filesystem). Because fuzzer is differential, only 1 filesystem with coverage support is enough, although not as effective.
- __Kernel Version Agnostic__ - only "hard" kernel dependency is __KCov__ feature.

## Build

### Native

Install rust.

Build:

```sh
cargo build --release
```

Compiled binaries will be put in `./target/release/...`

### Docker

Because binaries compiled on systems with __new__ `glibc` cannot be run on systems with __old__ `glibc` you might want to choose to compile with __docker__. This can be useful if running in VM.

Install docker.

Build image:

```sh
docker build . -t diffuzzer-builder
```

Run image:

```sh
docker run -v .:/usr/src diffuzzer-builder build --release
```

Compiled binaries will be put in `./target/release/...`

## Configuration

Configure with:

- [fuzzer configuration file](./config.toml) in TOML format
- [logging configuration file](./log4rs.yml) in YAML format ([docs](https://docs.rs/log4rs/latest/log4rs/#configuration)).

## QEMU

Read [QEMU configuration](./docs/QEMU.md) docs.

## Usage

For usage:

```sh
./target/release/diffuzzer --help
```

DIFFuzzer comes with many modes:

- greybox - greybox fuzzing (with coverage and mutations)
- blackbox - blackbox fuzzing
- single - run single test
- reduce - reduce testcase with bug

```sh
./target/release/diffuzzer greybox -f ext4 -s btrfs    # QEMU
./target/release/diffuzzer -n greybox -f ext4 -s btrfs # native
```

## Adding New Filesystem

Implement [trait](./diffuzzer/src/mount/mod.rs) (interface) for mounting filesystem. Default implementation uses `mkfs` and `mount` and can be used for most kernel filesystems (e.g. Ext4, Btrfs).

Add filesystem to [this file](./diffuzzer/src/filesystems.rs).

Done!

## Bugs Found

>TODO: bugs

## License

All the code is licensed under the "Mozilla Public License Version 2.0", unless specified otherwise.
