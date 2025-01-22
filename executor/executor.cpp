#include "executor.h"

#include <dirent.h>
#include <fcntl.h>
#include <linux/types.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/statfs.h>
#include <sys/types.h>
#include <sys/xattr.h>
#include <unistd.h>

#include <cassert>
#include <cerrno>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <string>
#include <vector>

#define KCOV_INIT_TRACE _IOR('c', 1, unsigned long)
#define KCOV_ENABLE _IO('c', 100)
#define KCOV_DISABLE _IO('c', 101)
#define COVER_SIZE (64 << 10)

#define KCOV_TRACE_PC 0
#define KCOV_TRACE_CMP 1

#define DPRINTF(...)                                \
  do {                                              \
    fprintf(stderr, "%s:%d: ", __FILE__, __LINE__); \
    fprintf(stderr, __VA_ARGS__);                   \
    fprintf(stderr, "\n");                          \
  } while (0)

#define GOAL(...)        \
  do {                   \
    printf(":: ");       \
    printf(__VA_ARGS__); \
    printf("\n");        \
  } while (0)

#define SUBGOAL(...)     \
  do {                   \
    printf("==> ");      \
    printf(__VA_ARGS__); \
    printf("\n");        \
  } while (0)

const char *MKDIR = "MKDIR";
const char *RMDIR = "RMDIR";
const char *CREATE = "CREATE";
const char *CLOSE = "CLOSE";
const char *UNLINK = "UNLINK";
const char *STAT = "STAT";
const char *HARDLINK = "HARDLINK";
const char *RENAME = "RENAME";
const char *OPEN = "OPEN";

enum ExitCode : int {
  OK = 0,
  FAIL = 1,

  ERROR = 2,
};

struct Trace {
  int idx;
  std::string cmd;
  int ret_code;
  int err;
};

std::vector<Trace> traces;

static void append_trace(int idx, const char *cmd, int ret_code, int err) {
  traces.push_back(Trace{idx, cmd, ret_code, err});
}

const char *workspace = nullptr;

static int failure_n = 0;
static int success_n = 0;

int main(int argc, char *argv[]) {
  if (argc != 2) {
    DPRINTF("[USAGE] CMD <workspace>");
    return ERROR;
  }

  workspace = argv[1];
  if (!workspace) {
    DPRINTF("[ERROR] <workspace> argument is NULL");
    return ERROR;
  }

  GOAL("prepare workspace '%s'", workspace);
  SUBGOAL("mkdir '%s'", workspace);
  if (mkdir(workspace, S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH) == -1) {
    if (errno == EEXIST) {
      DPRINTF("[WARNING] directory '%s' exists", workspace);
    } else {
      DPRINTF("[ERROR] %s", strerror(errno));
      return ERROR;
    }
  }

  GOAL("set up kcov");
  // https://docs.kernel.org/dev-tools/kcov.html
  bool coverage_enabled = true;
  int kcov_filed;
  unsigned long *cover;
  kcov_filed = open("/sys/kernel/debug/kcov", O_RDWR);
  if (kcov_filed == -1) {
    DPRINTF("[WARNING] failed to open kcov file, coverage disabled");
    coverage_enabled = false;
  } else {
    // setup trace mode and trace size
    if (ioctl(kcov_filed, KCOV_INIT_TRACE, COVER_SIZE)) {
      DPRINTF("[ERROR] failed to setup trace mode (ioctl)");
      return ERROR;
    }
    // mmap buffer shared between kernel- and user-space
    cover = (unsigned long *)mmap(nullptr, COVER_SIZE * sizeof(unsigned long),
                                  PROT_READ | PROT_WRITE, MAP_SHARED,
                                  kcov_filed, 0);
    if ((void *)cover == MAP_FAILED) {
      DPRINTF("[ERROR] failed to mmap coverage buffer");
      return ERROR;
    }
    // enable coverage collection on the current thread
    if (ioctl(kcov_filed, KCOV_ENABLE, KCOV_TRACE_PC)) {
      DPRINTF("[ERROR] failed to enable coverage collection (ioctl)");
      return ERROR;
    }
    // reset coverage from the tail of the ioctl() call
    __atomic_store_n(&cover[0], 0, __ATOMIC_RELAXED);
    SUBGOAL("done");
  }

  GOAL("test workload");
  test_workload();
  SUBGOAL("done");

  if (coverage_enabled) {
    GOAL("disable coverage collection");
    if (ioctl(kcov_filed, KCOV_DISABLE, 0)) {
      DPRINTF("[ERROR] when disabling coverage collection");
      return ERROR;
    }
    GOAL("dump kcov coverage");
    // read number of PCs collected
    std::filesystem::path kcov_p = "kcov.dat";
    FILE *trace_dump_fp = fopen(kcov_p.c_str(), "w");
    if (!trace_dump_fp) {
      DPRINTF("[ERROR] when opening kcov dump file: %s", strerror(errno));
      return ERROR;
    }
    unsigned long n = __atomic_load_n(&cover[0], __ATOMIC_RELAXED);
    for (unsigned long i = 0; i < n; i++) {
      fprintf(trace_dump_fp, "0x%lx\n", cover[i + 1]);
    }
    if (!fclose(trace_dump_fp)) {
      SUBGOAL("kcov dump saved at '%s'",
              std::filesystem::absolute(kcov_p).c_str());
    } else {
      DPRINTF("[ERROR] when closing kcov dump file: %s", strerror(errno));
      return ERROR;
    }
    GOAL("free kcov resources");
    if (munmap(cover, COVER_SIZE * sizeof(unsigned long))) {
      DPRINTF("[ERROR] when unmapping shared buffer");
      return ERROR;
    }
    if (close(kcov_filed)) {
      DPRINTF("[ERROR] when closing kcov file");
      return ERROR;
    }
    SUBGOAL("done");
  }

  GOAL("dump trace");
  std::filesystem::path trace_p = "trace.csv";
  FILE *trace_dump_fp = fopen(trace_p.c_str(), "w");
  if (!trace_dump_fp) {
    DPRINTF("[ERROR] when opening trace dump file: %s", strerror(errno));
    return ERROR;
  }
  fprintf(trace_dump_fp, "Index,Command,ReturnCode,Errno\n");
  for (const Trace &t : traces) {
    fprintf(trace_dump_fp, "%4d,%12s,%8d,%s(%d)\n", t.idx, t.cmd.c_str(),
            t.ret_code, strerror(t.err), t.err);
  }
  if (!fclose(trace_dump_fp)) {
    SUBGOAL("trace dump saved at '%s'",
            std::filesystem::absolute(trace_p).c_str());
  } else {
    DPRINTF("[ERROR] when closing trace dump file: %s", strerror(errno));
    return ERROR;
  }

  GOAL("summary");
  printf("#SUCCESS: %d | #FAILURE: %d\n", success_n, failure_n);
  if (failure_n > 0) {
    return FAIL;
  }
  return OK;
}

static std::string patch_path(const std::string &path) {
  if (path[0] != '/') {
    DPRINTF("[ERROR] when patching path '%s', expected path to start with '/'",
            path.c_str());
    exit(ERROR);
  }
  return workspace + path;
}

static std::string path_join(const std::string &prefix,
                             const std::string &file_name) {
  return prefix + "/" + file_name;
}

static int idx = -1;

static void success(int status, const char *cmd) {
  append_trace(idx, cmd, status, 0);
  success_n += 1;
}

static void failure(int status, const char *cmd, const char *path) {
  append_trace(idx, cmd, status, errno);
  DPRINTF("[WARNING] %s('%s') FAIL(%s)", cmd, path, strerror(errno));
  failure_n += 1;
}

static void failure2(int status, const char *cmd, const char *fst_path,
                     const char *snd_path) {
  append_trace(idx, cmd, status, errno);
  DPRINTF("[WARNING] %s('%s', '%s') FAIL(%s)", cmd, fst_path, snd_path,
          strerror(errno));
  failure_n += 1;
}

static void minor_failure(const char *cmd, const char *path) {
  DPRINTF("[WARNING] %s('%s') FAIL(%s) <minor>", cmd, path, strerror(errno));
}

int do_mkdir(const char *path, mode_t param) {
  idx++;
  int status = mkdir(patch_path(path).c_str(), param);
  if (status == -1) {
    failure(status, MKDIR, path);
  } else {
    success(status, MKDIR);
  }
  return status;
}

int do_create(const char *path, mode_t param) {
  idx++;
  int status = creat(patch_path(path).c_str(), param);
  if (status == -1) {
    failure(status, CREATE, path);
  } else {
    int close_status = close(status);
    if (!close_status) {
      success(status, CREATE);
    } else {
      minor_failure(CLOSE, path);
      failure(status, CREATE, path);
    }
  }
  return status;
}

static int remove_dir(const char *p) {
  const std::string dir_path(p);
  DIR *d = opendir(dir_path.c_str());
  int status = -1;

  if (d) {
    struct dirent *p;
    status = 0;

    while (!status && (p = readdir(d))) {
      if (!strcmp(p->d_name, ".") || !strcmp(p->d_name, "..")) {
        continue;
      }

      struct stat statbuf;
      int status_in_dir = -1;
      const std::string file_path = path_join(dir_path, p->d_name);

      if (!lstat(file_path.c_str(), &statbuf)) {
        if (S_ISDIR(statbuf.st_mode)) {
          status_in_dir = remove_dir(file_path.c_str());
        } else {
          status_in_dir = unlink(file_path.c_str());
          if (status_in_dir) {
            minor_failure(UNLINK, file_path.c_str());
          }
        }
      }
      status = status_in_dir;
    }
    closedir(d);
  }

  if (!status) {
    status = rmdir(dir_path.c_str());
  }

  if (status) {
    minor_failure(RMDIR, dir_path.c_str());
  }

  return status;
}

int do_remove(const char *p) {
  idx++;
  const std::string path = patch_path(p);
  struct stat file_stat;
  int status = 0;

  status = lstat(path.c_str(), &file_stat);
  if (status < 0) {
    failure(status, STAT, path.c_str());
    return -1;
  }

  if (S_ISDIR(file_stat.st_mode)) {
    status = remove_dir(path.c_str());
    if (status) {
      failure(status, RMDIR, path.c_str());
    } else {
      success(status, RMDIR);
    }
  } else {
    status = unlink(path.c_str());
    if (status == -1) {
      failure(status, UNLINK, path.c_str());
    } else {
      success(status, UNLINK);
    }
  }

  return status;
}

int do_hardlink(const char *old_path, const char *new_path) {
  idx++;
  int status = link(patch_path(old_path).c_str(), patch_path(new_path).c_str());
  if (status == -1) {
    failure2(status, HARDLINK, old_path, new_path);
  } else {
    success(status, HARDLINK);
  }
  return status;
}

int do_rename(const char *old_path, const char *new_path) {
  idx++;
  int status =
      rename(patch_path(old_path).c_str(), patch_path(new_path).c_str());
  if (status == -1) {
    failure2(status, RENAME, old_path, new_path);
  } else {
    success(status, RENAME);
  }
  return status;
}

int do_open(const char *path) {
  idx++;
  int fd = open(patch_path(path).c_str(), O_RDWR);
  if (fd == -1) {
    failure(fd, OPEN, path);
  } else {
    success(fd, OPEN);
  }
  return fd;
}

int do_close(int fd) {
  idx++;
  int status = close(fd);
  if (status == -1) {
    failure(status, CLOSE, std::to_string(fd).c_str());
  } else {
    success(status, CLOSE);
  }
  return status;
}
