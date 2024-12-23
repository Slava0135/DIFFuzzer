use crate::abstract_fs::{
    self,
    types::{Mode, Operation, Workload},
};

impl Workload {
    pub fn encode_c(&self) -> String {
        let mut result = String::new();
        result.push_str("#include \"executor.h\"\n");
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
    use abstract_fs::types::ModeFlag;

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
            ModeFlag::S_IRWXU,
            ModeFlag::S_IRWXG,
            ModeFlag::S_IROTH,
            ModeFlag::S_IXOTH,
        ];
        let actual = Workload {
            ops: vec![
                Operation::MKDIR {
                    path: "/foo".to_owned(),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: "/foo/bar".to_owned(),
                    mode: mode.clone(),
                },
                Operation::REMOVE {
                    path: "/foo".to_owned(),
                },
            ],
        }
        .encode_c();
        assert_eq!(expected, actual);
    }
}
