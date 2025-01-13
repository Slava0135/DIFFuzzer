#pragma once

#include <sys/stat.h>
#include <fcntl.h>
#include <cstdio>
#include <cstdlib>
#include <cerrno>
#include <cstring>

extern "C" {
    void test_workload();

    int do_mkdir(const char *path, mode_t param);
    int do_create(const char *path, mode_t param);
    int do_remove(const char *path);
    int do_hardlink(const char *old_path, const char *new_path);
}
