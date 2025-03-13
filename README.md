# DIFFuzzer - Differential Filesystem Fuzzer

__DIFFuzzer__ - is a fuzzer, that aims to find __memory__ and __semantic__ bugs in __kernel__ (Linux) and __userspace__ (FUSE) filesystems.

It expands on previous works, such as:

- [Hydra](https://dl.acm.org/doi/abs/10.1145/3341301.3359662), filesystem fuzzing framework.
- [Dogfood](https://dl.acm.org/doi/abs/10.1145/3377811.3380350), filsystem test workload generator.
- [CrashMonkey](https://dl.acm.org/doi/abs/10.1145/3320275), filesystem crash consistency testing framework.
- [Metis](https://www.usenix.org/conference/fast24/presentation/liu-yifei), filesystem model checking tool.
- [SibylFS](https://dl.acm.org/doi/abs/10.1145/2815400.2815411), oracle-based testing for filesystems.
- and other...

Key features:

- __Filesystem Semantics__ - to generate "good" inputs, filesystem semantics must be modelled properly (as was shown in [Hydra](https://dl.acm.org/doi/abs/10.1145/3341301.3359662)).
- __Differential__ - two filesystems are tested against same input and differences in their execution are observed in order to discover __semantic__ bugs.
- __Coverage Guided__ - similar to [Syzkaller](https://github.com/google/syzkaller), kernel coverage (__KCov__) is used to pick and mutate "interesting" inputs.
- __Native and QEMU__ - can be run on local machine as well as in VM using __QEMU__.
- __FUSE Supported__ - can be used for testing __FUSE__ file systems using __LCov__ coverage information (can run without coverage, but not as effective).
- __Easy Filesystem Integration__ - see [Adding New Filesystem](#adding-new-filesystem).
- __Kernel Version Agnostic__ - only __KCov__ is required.

## Structure

Project consists of 4 parts:

- `diffuzzer` - fuzzer itself.
- `dash` - differential abstract state hasher, used for evaluating and comparing file system states.
- `executor` - runtime/library that is used by tests.
- `tools` - miscellaneous scripts.

## Build

### Native

Install rust.

Build project:

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

- [Fuzzer configuration file](./config.toml) in TOML format ([docs](./diffuzzer/src/config.rs)).
- [Logging configuration file](./log4rs.yml) in YAML format ([docs](https://docs.rs/log4rs/latest/log4rs/#configuration)).

## QEMU

Read [QEMU configuration](./docs/QEMU.md) docs.

> __You need to configure QEMU image before running fuzzer.__

## Usage

For usage:

```sh
./target/release/diffuzzer --help
```

DIFFuzzer comes with many modes:

- `greybox` - greybox fuzzing (with coverage and mutations)
- `blackbox` - blackbox fuzzing
- `reduce` - reduce testcase with bug
- `solo-single` - run single test
- `duo-single` - run single test for 2 filesystems

> __For greybox fuzzing, kernel instrumented with KCov is required.__

```sh
./target/release/diffuzzer greybox -f ext4 -s btrfs
```

There is also an option to run without QEMU (*not recommended*):

```sh
./target/release/diffuzzer -n greybox -f ext4 -s btrfs
```

## Adding New Filesystem

Implement [trait](./diffuzzer/src/mount/mod.rs) (interface) for mounting filesystem. Default implementation uses `mkfs` and `mount` and can be used for most kernel filesystems (e.g. Ext4, Btrfs).

Add your filesystem to [this file](./diffuzzer/src/filesystems.rs).

Done!

For additional information read [Filesystems](./docs/Filesystems.md) docs.

## Bugs Found

>TBD

## License

All the code is licensed under the "Mozilla Public License Version 2.0", unless specified otherwise.
