use std::cmp::max;

use super::{flags::Mode, node::FileDescriptor, operation::Operation, workload::Workload};

fn descriptor_to_var(des: &FileDescriptor) -> String {
    format!("fd_{}", des.0)
}

impl Workload {
    pub fn encode_c(&self) -> String {
        let mut result = String::new();
        result.push_str("#include \"executor.h\"\n");
        let mut descriptors_n = 0;
        for op in self.ops.iter() {
            match op {
                Operation::OPEN { path: _, des } => {
                    descriptors_n = max(descriptors_n, des.0 + 1);
                }
                _ => {}
            }
        }
        if descriptors_n > 0 {
            let descriptors_vars: Vec<String> =
                (0..descriptors_n).map(|it| format!("fd_{}", it)).collect();
            result.push_str(format!("\nint {};\n\n", descriptors_vars.join(", ")).as_str());
        } else {
            result.push_str("\n// no descriptors\n\n");
        }
        result.push_str("void test_workload()\n");
        result.push_str("{\n");
        for op in &self.ops {
            match op {
                Operation::CREATE { path, mode } => {
                    result.push_str(
                        format!("do_create(\"{}\", {});\n", path, encode_mode(mode).as_str())
                            .as_str(),
                    );
                }
                Operation::MKDIR { path, mode } => {
                    result.push_str(
                        format!("do_mkdir(\"{}\", {});\n", path, encode_mode(mode).as_str())
                            .as_str(),
                    );
                }
                Operation::REMOVE { path } => {
                    result.push_str(format!("do_remove(\"{}\");\n", path).as_str());
                }
                Operation::HARDLINK { old_path, new_path } => {
                    result.push_str(
                        format!("do_hardlink(\"{}\", \"{}\");\n", old_path, new_path).as_str(),
                    );
                }
                Operation::RENAME { old_path, new_path } => {
                    result.push_str(
                        format!("do_rename(\"{}\", \"{}\");\n", old_path, new_path).as_str(),
                    );
                }
                Operation::OPEN { path, des } => {
                    result.push_str(
                        format!("{} = do_open(\"{}\");\n", descriptor_to_var(des), path).as_str(),
                    );
                }
                Operation::CLOSE { des } => {
                    result.push_str(format!("do_close({});\n", descriptor_to_var(des)).as_str());
                }
                Operation::READ { des, size } => {
                    result.push_str(
                        format!("do_read({}, {});\n", descriptor_to_var(des), size).as_str(),
                    );
                }
            }
        }
        result.push_str("}");
        result
    }
}

fn encode_mode(mode: &Mode) -> String {
    if mode.is_empty() {
        0.to_string()
    } else {
        let mode_str: Vec<String> = mode.iter().map(|mf| mf.to_string()).collect();
        mode_str.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use crate::abstract_fs::{flags::ModeFlag, node::FileDescriptor};

    use super::*;

    #[test]
    fn test_encode_c_empty() {
        let expected = r#"
#include "executor.h"

// no descriptors

void test_workload()
{
}
"#
        .trim();
        let actual = Workload { ops: vec![] }.encode_c();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_encode_c() {
        let expected = r#"
#include "executor.h"

int fd_0, fd_1;

void test_workload()
{
do_mkdir("/foo", 0);
do_create("/foo/bar", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
fd_0 = do_open("/foo/bar");
do_read(fd_0, 1024);
do_close(fd_0);
do_hardlink("/foo/bar", "/baz");
fd_1 = do_open("/baz");
do_close(fd_1);
do_rename("/baz", "/gaz");
do_remove("/foo");
}
"#
        .trim();
        let mode = vec![
            ModeFlag::S_IRWXU,
            ModeFlag::S_IRWXG,
            ModeFlag::S_IROTH,
            ModeFlag::S_IXOTH,
        ];
        let actual = Workload {
            ops: vec![
                Operation::MKDIR {
                    path: "/foo".into(),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: "/foo/bar".into(),
                    mode: mode.clone(),
                },
                Operation::OPEN {
                    path: "/foo/bar".into(),
                    des: FileDescriptor(0),
                },
                Operation::READ {
                    des: FileDescriptor(0),
                    size: 1024,
                },
                Operation::CLOSE {
                    des: FileDescriptor(0),
                },
                Operation::HARDLINK {
                    old_path: "/foo/bar".into(),
                    new_path: "/baz".into(),
                },
                Operation::OPEN {
                    path: "/baz".into(),
                    des: FileDescriptor(1),
                },
                Operation::CLOSE {
                    des: FileDescriptor(1),
                },
                Operation::RENAME {
                    old_path: "/baz".into(),
                    new_path: "/gaz".into(),
                },
                Operation::REMOVE {
                    path: "/foo".into(),
                },
            ],
        }
        .encode_c();
        assert_eq!(expected, actual);
    }
}
