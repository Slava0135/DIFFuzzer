#include "executor.h"

int fd_0, fd_1;

void test_workload()
{
do_mkdir("/foo", 0);
do_create("/foo/bar", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_0 = do_open("/foo/bar");
do_write(fd_0, 999, 1024);
do_close(fd_0);
do_hardlink("/foo/bar", "/baz");
fd_1 = do_open("/baz");
do_read(fd_1, 1024);
do_close(fd_1);
do_rename("/baz", "/gaz");
do_remove("/foo");
}