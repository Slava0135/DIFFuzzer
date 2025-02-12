#pragma once

#include <fcntl.h>
#include <sys/stat.h>

#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>

extern "C" {
void test_workload();

/// `mkdir` operation.
int do_mkdir(const char *path, mode_t param);
/// `creat` operation, but file descriptor is closed immediately.
int do_create(const char *path, mode_t param);
/// `unlink` for files + `rmdir` for directories (after all files are deleted).
int do_remove(const char *path);
/// `link` operation.
int do_hardlink(const char *old_path, const char *new_path);
/// `rename` operation.
int do_rename(const char *old_path, const char *new_path);
/// `open` operation. TODO: flags
int do_open(const char *path);
/// `close` operation.
int do_close(int fd);
/// `write` operation, but instead of char buffer, position inside some "source" buffer is used.
int do_write(int fd, size_t src_offset, size_t size);
/// `read` operation, but same read buffer is used.
int do_read(int fd, size_t size);
/// `fsync` operation.
int do_fsync(int fd);
}
