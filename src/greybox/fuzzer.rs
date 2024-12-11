use std::{
    env,
    num::NonZero,
    path::{Path, PathBuf},
    time::Duration,
};

use libafl::{
    corpus::{InMemoryCorpus, OnDiskCorpus},
    events::SimpleEventManager,
    executors::{DiffExecutor, InProcessExecutor},
    feedback_or,
    monitors::SimpleMonitor,
    schedulers::QueueScheduler,
    stages::StdMutationalStage,
    state::StdState,
    Error, Fuzzer, StdFuzzer,
};
use libafl_bolts::{
    current_nanos,
    rands::StdRand,
    tuples::{tuple_list, Handled},
};
use log::{error, info};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    abstract_fs::types::Workload,
    mount::{btrfs::Btrfs, ext4::Ext4},
};

use super::{
    executor::workload_harness,
    feedback::kcov::KCovFeedback,
    input::WorkloadMutator,
    objective::trace::TraceObjective,
    observer::{kcov::KCovObserver, trace::TraceObserver},
};

pub fn fuzz() {
    let temp_dir = env::temp_dir().join("DIFFuzzer");
    std::fs::remove_dir_all(temp_dir.as_path()).unwrap_or(());
    std::fs::create_dir(temp_dir.as_path()).unwrap_or(());

    let trace_path = temp_dir.join("trace.csv");
    let kcov_path = temp_dir.join("kcov.dat");

    let fst_trace_observer = TraceObserver::new(trace_path.clone().into_boxed_path());
    let snd_trace_observer = TraceObserver::new(trace_path.clone().into_boxed_path());

    let fst_kcov_observer = KCovObserver::new(kcov_path.clone().into_boxed_path());
    let snd_kcov_observer = KCovObserver::new(kcov_path.clone().into_boxed_path());

    let fst_kcov_feedback = KCovFeedback::new(fst_kcov_observer.handle());
    let snd_kcov_feedback = KCovFeedback::new(snd_kcov_observer.handle());

    let mut feedback = feedback_or!(fst_kcov_feedback, snd_kcov_feedback);

    let mut objective =
        TraceObjective::new(fst_trace_observer.handle(), snd_trace_observer.handle());

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        // Corpus that will be evolved, we keep it in memory for performance
        InMemoryCorpus::<Workload>::new(),
        // Corpus in which we store solutions (crashes in this example),
        // on disk so the user can get them after stopping the fuzzer
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(),
        // States of the feedbacks.
        // The feedbacks can report the data that should persist in the State.
        &mut feedback,
        // Same for objective feedbacks
        &mut objective,
    )
    .unwrap();

    let monitor = SimpleMonitor::new(|s| info!("{s}"));

    let mut manager = SimpleEventManager::new(monitor);

    let scheduler = QueueScheduler::new();

    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let mut fst_harness = workload_harness(
        Ext4::new(),
        Path::new("/mnt")
            .join("ext4")
            .join("fstest")
            .into_boxed_path(),
        temp_dir.clone().into_boxed_path(),
    );

    let mut snd_harness = workload_harness(
        Btrfs::new(),
        Path::new("/mnt")
            .join("btrfs")
            .join("fstest")
            .into_boxed_path(),
        temp_dir.clone().into_boxed_path(),
    );

    let timeout = Duration::new(10, 0);

    let fst_executor = InProcessExecutor::with_timeout(
        &mut fst_harness,
        tuple_list!(fst_kcov_observer, fst_trace_observer),
        &mut fuzzer,
        &mut state,
        &mut manager,
        timeout,
    )
    .unwrap();

    let snd_executor = InProcessExecutor::with_timeout(
        &mut snd_harness,
        tuple_list!(snd_kcov_observer, snd_trace_observer),
        &mut fuzzer,
        &mut state,
        &mut manager,
        timeout,
    )
    .unwrap();

    let mut executor = DiffExecutor::new(fst_executor, snd_executor, tuple_list!());

    let mutator = WorkloadMutator::new(StdRng::seed_from_u64(current_nanos()));
    let mut stages = tuple_list!(StdMutationalStage::with_max_iterations(
        mutator,
        NonZero::new(10).unwrap()
    ));

    loop {
        match fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut manager) {
            Ok(_) => break,
            Err(Error::ShuttingDown) => break,
            Err(err) => error!("{err:?}"),
        }
    }
}
