#pragma once

#include <fcntl.h>
#include <sys/stat.h>

#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <cstring>

extern "C" {
void test_workload();

int do_mkdir(const char *path, mode_t param);
int do_create(const char *path, mode_t param);
int do_remove(const char *path);
int do_hardlink(const char *old_path, const char *new_path);
int do_rename(const char *old_path, const char *new_path);
int do_open(const char *path);
int do_close(int fd);
int do_write(int fd, size_t src_offset, size_t size);
int do_read(int fd, size_t size);
}
