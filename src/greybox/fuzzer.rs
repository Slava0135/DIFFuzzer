use std::{
    cell::RefCell,
    path::Path,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use log::info;
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    abstract_fs::types::Workload,
    config::Config,
    greybox::objective::{console::ConsoleObjective, trace::TraceObjective},
    mount::{btrfs::Btrfs, ext4::Ext4},
    temp_dir::setup_temp_dir,
};

use super::{feedback::kcov::KCovFeedback, harness::Harness, mutator::Mutator};

pub struct Fuzzer {
    corpus: Vec<Workload>,
    next_seed: usize,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    trace_objective: TraceObjective,
    console_objective: ConsoleObjective,

    fst_harness: Harness<Ext4>,
    snd_harness: Harness<Btrfs>,

    mutator: Mutator,
}

impl Fuzzer {
    pub fn new(config: Config) -> Self {
        info!("new greybox fuzzer");

        info!("setting up temporary directory");
        let temp_dir = setup_temp_dir();

        info!("setting up fuzzing components");
        let test_dir = temp_dir.clone();
        let exec_dir = temp_dir.join("exec");
        let trace_path = exec_dir.join("trace.csv");
        let kcov_path = exec_dir.join("kcov.dat");

        let fst_stdout = Rc::new(RefCell::new("".to_owned()));
        let fst_stderr = Rc::new(RefCell::new("".to_owned()));
        let snd_stdout = Rc::new(RefCell::new("".to_owned()));
        let snd_stderr = Rc::new(RefCell::new("".to_owned()));

        let fst_kcov_feedback = KCovFeedback::new(kcov_path.clone().into_boxed_path());
        let snd_kcov_feedback = KCovFeedback::new(kcov_path.clone().into_boxed_path());

        let trace_objective = TraceObjective::new(
            trace_path.clone().into_boxed_path(),
            trace_path.clone().into_boxed_path(),
        );
        let console_objective = ConsoleObjective::new(
            fst_stdout.clone(),
            fst_stderr.clone(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        let fst_harness = Harness::new(
            Ext4::new(),
            Path::new("/mnt")
                .join("ext4")
                .join("fstest")
                .into_boxed_path(),
            test_dir.clone().into_boxed_path(),
            exec_dir.clone().into_boxed_path(),
            fst_stdout,
            fst_stderr,
        );
        let snd_harness = Harness::new(
            Btrfs::new(),
            Path::new("/mnt")
                .join("btrfs")
                .join("fstest")
                .into_boxed_path(),
            test_dir.clone().into_boxed_path(),
            exec_dir.clone().into_boxed_path(),
            snd_stdout,
            snd_stderr,
        );

        let mutator = Mutator::new(
            StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
            config.operation_weights.clone(),
            config.mutation_weights.clone(),
            config.greybox.max_workload_length,
        );

        Self {
            corpus: vec![Workload::new()],
            next_seed: 0,

            fst_kcov_feedback,
            snd_kcov_feedback,

            trace_objective,
            console_objective,

            fst_harness,
            snd_harness,

            mutator,
        }
    }

    pub fn fuzz(self) {
        info!("starting fuzzing loop");
    }
}
