#include "executor.h"

int fd_0, fd_1;

void test_workload()
{
do_create("/1", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_0 = do_open("/1");
do_rename("/1", "/2");
do_write(fd_0, 0, 10);
do_close(fd_0);
fd_1 = do_open("/2");
do_read(fd_1, 10);
}