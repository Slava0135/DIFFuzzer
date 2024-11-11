#include "executor.h"

#include <dirent.h>
#include <fcntl.h>
#include <stdlib.h>
#include <sys/ioctl.h>
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

#define DPRINTF(...)                                \
  do {                                              \
    fprintf(stderr, "%s:%d: ", __FILE__, __LINE__); \
    fprintf(stderr, __VA_ARGS__);                   \
  } while (0)

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
    DPRINTF("[USAGE] CMD <workspace>\n");
    return 1;
  }

  workspace = argv[1];
  if (!workspace) {
    DPRINTF("[ERROR] <workspace> argument is NULL\n");
    return 1;
  }

  fprintf(stdout, ":: preparing workspace '%s'\n", workspace);
  fprintf(stdout, "==> mkdir '%s'\n", workspace);
  if (mkdir(workspace, S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH) == -1) {
    if (errno == EEXIST) {
      DPRINTF("[WARNING] directory '%s' exists\n", workspace);
    } else {
      DPRINTF("[ERROR] %s\n", strerror(errno));
      return 1;
    }
  }

  fprintf(stdout, ":: testing workload\n");
  test_workload();
  fprintf(stdout, "==> done\n");

  fprintf(stdout, ":: dumping trace\n");
  std::filesystem::path trace_p = "trace.csv";
  FILE *trace_dump_fp = fopen(trace_p.c_str(), "w");
  if (!trace_dump_fp) {
    DPRINTF("[ERROR] when opening trace dump file: %s\n", strerror(errno));
    return 1;
  }
  fprintf(trace_dump_fp, "Index,Command,ReturnCode,Errno\n");
  for (const Trace &t : traces) {
    fprintf(trace_dump_fp, "%4d,%12s,%8d,%s(%d)\n", t.idx, t.cmd.c_str(),
            t.ret_code, strerror(t.err), t.err);
  }
  if (!fclose(trace_dump_fp)) {
    fprintf(stdout, "==> trace dump saved at '%s'\n",
            std::filesystem::absolute(trace_p).c_str());
  } else {
    DPRINTF("[ERROR] when closing trace dump file: %s\n", strerror(errno));
  }

  fprintf(stdout, ":: run summary\n");
  fprintf(stdout, "#SUCCESS: %d | #FAILURE: %d\n", success_n, failure_n);
  return 1;
}

static std::string patch_path(const std::string &path) {
  if (path[0] != '/') {
    DPRINTF(
        "[ERROR] when patching path '%s', expected path to start with '\\'\n",
        path.c_str());
    exit(1);
  }
  return workspace + path;
}

static std::string path_join(const std::string &prefix,
                             const std::string &file_name) {
  return prefix + "/" + file_name;
}

static int idx = 0;

static void success(int status, const char *cmd) {
  append_trace(idx, cmd, status, 0);
  success_n += 1;
}

static void failure(int status, const char *cmd, const char *path) {
  append_trace(idx, cmd, status, errno);
  DPRINTF("[WARNING] %s('%s') FAILED (%s)\n", cmd, path, strerror(errno));
  failure_n += 1;
}

int do_mkdir(const char *path, mode_t param) {
  idx++;
  int status = mkdir(patch_path(path).c_str(), param);
  if (status == -1) {
    failure(status, "MKDIR", path);
  } else {
    success(status, "MKDIR");
  }
  return status;
}

int do_create(const char *path, mode_t param) {
  idx++;
  int status = creat(patch_path(path).c_str(), param);
  if (status == -1) {
    failure(status, "CREATE", path);
  } else {
    success(status, "CREATE");
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
            DPRINTF("[ERROR] unlink('%s') failure\n", file_path.c_str());
          }
        }
      }
      status = status_in_dir;
    }
    closedir(d);
  }

  if (!status) {
    status = rmdir(dir_path.c_str());
  } else {
    DPRINTF("[ERROR] rmdir('%s') failure\n", dir_path.c_str());
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
    failure(status, "STAT", path.c_str());
    return -1;
  }

  if (S_ISDIR(file_stat.st_mode)) {
    status = remove_dir(path.c_str());
    if (status) {
      failure(status, "RMDIR", path.c_str());
    } else {
      success(status, "RMDIR");
    }
  } else {
    status = unlink(path.c_str());
    if (status == -1) {
      failure(status, "UNLINK", path.c_str());
    } else {
      success(status, "UNLINK");
    }
  }

  return status;
}
