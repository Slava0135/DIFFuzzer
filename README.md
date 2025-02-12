# DIFFuzzer
## build with docker
```shell
docker build . -t diffuzzer-builder
```
```shell
docker run -v .:/usr/src diffuzzer-builder build --release
```