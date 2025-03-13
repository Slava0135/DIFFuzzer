# Data is lost if file with open descriptor is renamed before writing

[Issue #78](https://github.com/littlefs-project/littlefs-fuse/issues/78)

## Description

```c
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

int main() {
  char buffer[100] = {0};
  int fd_0 = creat("1", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
  rename("1", "2");
  write(fd_0, "0123456789", 10);
  close(fd_0);
  int fd_1 = open("2", O_RDWR);
  int nread = read(fd_1, buffer, 10);
  printf("%d(%s)\n", nread, strerror(errno));
  printf("%s\n", buffer);
}

// ::Expected::
// 10(Success)
// 0123456789

// ::Actual::
// 0(Success)
//
```

If file is renamed when there is an open file descriptor, and any data is written, it will not be saved to disk.

Instead of using `read` at the end `cat 2` can be called in shell to verify data is missing.

## Version

```text
DIFFuzzer 66bc9a80e5356317031c21343346f5de5d708f65
LittleFS-FUSE 2cc2af5030f8bf831cd8355bc4780a34acbf6faa (tag: v2.7.10)
Ubuntu 22.04.5 LTS
Kernel 5.15.178
gcc 11.4.0
```
