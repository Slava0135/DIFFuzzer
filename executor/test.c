#include "executor.h"
void test_workload()
{
do_mkdir("/1", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/1/2", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_mkdir("/1/3", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/1/3/4", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_mkdir("/1/5", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/1/6", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_remove("/1/3/4");
do_create("/1/7", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_create("/1/3/8", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_remove("/1");
}