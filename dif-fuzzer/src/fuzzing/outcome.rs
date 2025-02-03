use std::process::ExitStatus;

use crate::path::LocalPath;

pub struct Outcome {
    pub exit_status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub dir: LocalPath,
}
