#include "executor.h"

int fd_0, fd_1, fd_2, fd_3, fd_4, fd_5, fd_6, fd_7, fd_8, fd_9, fd_10, fd_11, fd_12, fd_13, fd_14, fd_15, fd_16, fd_17, fd_18, fd_19, fd_20, fd_21, fd_22, fd_23, fd_24, fd_25, fd_26, fd_27, fd_28, fd_29, fd_30, fd_31, fd_32, fd_33, fd_34, fd_35, fd_36, fd_37, fd_38, fd_39, fd_40, fd_41, fd_42, fd_43, fd_44, fd_45, fd_46, fd_47, fd_48, fd_49, fd_50, fd_51, fd_52, fd_53, fd_54, fd_55, fd_56, fd_57, fd_58, fd_59, fd_60, fd_61, fd_62, fd_63, fd_64, fd_65, fd_66, fd_67, fd_68, fd_69, fd_70, fd_71, fd_72, fd_73, fd_74, fd_75, fd_76, fd_77, fd_78, fd_79, fd_80, fd_81, fd_82, fd_83, fd_84, fd_85, fd_86, fd_87, fd_88, fd_89, fd_90, fd_91, fd_92, fd_93, fd_94, fd_95, fd_96, fd_97, fd_98, fd_99, fd_100, fd_101, fd_102, fd_103, fd_104, fd_105, fd_106, fd_107, fd_108, fd_109, fd_110, fd_111, fd_112, fd_113, fd_114, fd_115, fd_116, fd_117, fd_118, fd_119, fd_120, fd_121, fd_122, fd_123, fd_124, fd_125, fd_126, fd_127, fd_128, fd_129, fd_130, fd_131, fd_132, fd_133, fd_134, fd_135, fd_136, fd_137, fd_138, fd_139, fd_140, fd_141;

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
