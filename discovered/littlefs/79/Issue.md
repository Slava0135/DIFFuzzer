# Removing directory with unlinked open file fails

[Issue #79](https://github.com/littlefs-project/littlefs-fuse/issues/79)

## Description

```c
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

int main() {
  int res;
  res = mkdir("1", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
  int fd = creat("1/2", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
  res = unlink("1/2");
  printf("UNLINK %d(%s)\n", res, strerror(errno));
  res = rmdir("1");
  printf("RMDIR %d(%s)\n", res, strerror(errno));
  res = close(fd);
  printf("CLOSE %d(%s)\n", res, strerror(errno));
}

// ::Expected::
// UNLINK 0(Success)
// RMDIR 0(Success)
// CLOSE 0(Success)

// ::Actual::
// UNLINK 0(Success)
// RMDIR -1(Directory not empty)
// CLOSE 0(Directory not empty)
```

If file with open file descriptor is deleted using `unlink`, `rmdir` will still fail.

## Version

```text
DIFFuzzer 66bc9a80e5356317031c21343346f5de5d708f65
LittleFS-FUSE 2cc2af5030f8bf831cd8355bc4780a34acbf6faa (tag: v2.7.10)
Ubuntu 22.04.5 LTS
Kernel 5.15.178
gcc 11.4.0
```
