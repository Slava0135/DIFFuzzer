#include "executor.h"

int fd_0, fd_1, fd_2, fd_3, fd_4;

void test_workload()
{
do_create("/0", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_rename("/0", "/1");
do_create("/2", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_0 = do_open("/2");
fd_1 = do_open("/1");
do_rename("/2", "/3");
do_write(fd_0, 1024, 1024);
do_write(fd_1, 100000, 32);
do_close(fd_0);
do_create("/4", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/5", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_read(fd_1, 100);
do_fsync(fd_1);
do_create("/6", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/7", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_read(fd_1, 65536);
do_close(fd_1);
do_rename("/3", "/8");
fd_2 = do_open("/5");
do_rename("/8", "/9");
fd_3 = do_open("/9");
do_read(fd_2, 65536);
do_write(fd_3, 65536, 128);
do_mkdir("/10", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_write(fd_3, 1024, 32);
do_mkdir("/10/11", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_4 = do_open("/7");
do_create("/10/11/12", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_close(fd_2);
do_remove("/1");
do_write(fd_4, 32768, 1000);
do_read(fd_3, 32);
}
