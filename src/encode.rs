use crate::abstract_fs::{self, Operation};

pub fn encode_c(workload: abstract_fs::Workload) -> String {
    let mut result = String::new();
    result.push_str("#include \"executor.h\"\n");
    result.push_str("void test_workload()\n");
    result.push_str("{\n");
    for op in workload {
        match op {
            Operation::CREATE { path, mode } => {
                result.push_str(
                    format!("do_create(\"{}\", {});\n", path, encode_mode(mode).as_str()).as_str(),
                );
            }
            Operation::MKDIR { path, mode } => {
                result.push_str(
                    format!("do_mkdir(\"{}\", {});\n", path, encode_mode(mode).as_str()).as_str(),
                );
            }
            Operation::REMOVE { path } => {
                result.push_str(format!("do_remove(\"{}\");\n", path).as_str());
            }
        }
    }
    result.push_str("}");
    result
}

fn encode_mode(mode: abstract_fs::Mode) -> String {
    if mode.is_empty() {
        0.to_string()
    } else {
        let mode_str: Vec<String> = mode.iter().map(|mf| mf.to_string()).collect();
        mode_str.join(" | ")
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_encode_c() {
        let expected = r#"
#include "executor.h"
void test_workload()
{
do_mkdir("/foo", 0);
do_create("/foo/bar", S_IRWXU | S_IRWXG | S_IROTH | S_IXOTH);
do_remove("/foo");
}
"#
        .trim();
        let mode = vec![
            abstract_fs::ModeFlag::S_IRWXU,
            abstract_fs::ModeFlag::S_IRWXG,
            abstract_fs::ModeFlag::S_IROTH,
            abstract_fs::ModeFlag::S_IXOTH,
        ];
        let actual = encode_c(vec![
            abstract_fs::Operation::MKDIR {
                path: String::from("/foo"),
                mode: vec![],
            },
            abstract_fs::Operation::CREATE {
                path: String::from("/foo/bar"),
                mode: mode.clone(),
            },
            abstract_fs::Operation::REMOVE {
                path: String::from("/foo"),
            },
        ]);
        assert_eq!(expected, actual);
    }
}
