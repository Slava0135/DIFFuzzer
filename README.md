# DIFFuzzer
## Build with min glibc version
```shell
docker container run --rm --volume "$(pwd)":/src     \
    --init --tty --user "$(id --user):$(id --group)" \
    unixgeek2/rust-min-libc build --release
```