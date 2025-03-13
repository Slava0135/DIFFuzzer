#include "executor.h"

int fd_0, fd_1, fd_2;

void test_workload()
{
do_mkdir("/5", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/5/9", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_0 = do_open("/5/9");
do_mkdir("/5/12", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_remove("/5");
do_create("/3", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_1 = do_open("/3");
do_create("/1", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_write(fd_0, 512, 16);
do_mkdir("/0", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_mkdir("/2", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_write(fd_0, 4096, 255);
do_mkdir("/11", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_write(fd_0, 127, 128);
do_write(fd_0, 32, 127);
do_read(fd_0, 127);
fd_2 = do_open("/1");
do_mkdir("/2/8", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_mkdir("/7", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_write(fd_1, 32, 100000);
do_mkdir("/0/10", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_rename("/2", "/6");
do_read(fd_2, 32768);
do_remove("/6");
do_write(fd_0, 65536, 255);
do_rename("/3", "/4");
}