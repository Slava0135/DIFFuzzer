use std::{sync::mpsc::Sender, time::Instant};

use anyhow::Context;
use log::{error, info};

use super::greybox::feedback::CoverageType;

pub enum BrokerMessage {
    Error { id: u8, err: anyhow::Error },
    BlackBoxStats { id: u8, stats: BlackBoxStats },
    GreyBoxStats { id: u8, stats: GreyBoxStats },
    Info { id: u8, msg: String },
}

#[derive(Clone)]
pub struct BlackBoxStats {
    pub crashes: u64,
    pub executions: u64,
}

impl BlackBoxStats {
    pub fn display(&self, start: &Instant) -> String {
        let secs = start.elapsed().as_secs();
        format!(
            "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.crashes,
            self.executions,
            (self.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        )
    }
}

#[derive(Clone)]
pub struct GreyBoxStats {
    pub corpus_size: u64,
    pub fst_coverage_size: u64,
    pub fst_coverage_type: CoverageType,
    pub snd_coverage_size: u64,
    pub snd_coverage_type: CoverageType,
    pub crashes: u64,
    pub executions: u64,
}

impl GreyBoxStats {
    pub fn display(&self, start: &Instant) -> String {
        let secs = start.elapsed().as_secs();
        format!(
            "corpus: {}, coverage: {} ({}) + {} ({}), crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.corpus_size,
            self.fst_coverage_size,
            self.fst_coverage_type,
            self.snd_coverage_size,
            self.snd_coverage_type,
            self.crashes,
            self.executions,
            (self.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        )
    }
}

pub enum InstanceMessage {
    Run { test_count: Option<u64> },
}

#[derive(Clone)]
pub enum BrokerHandle {
    Stub { start: Instant },
    Full { id: u8, tx: Sender<BrokerMessage> },
}

impl BrokerHandle {
    pub fn error(&self, err: anyhow::Error) -> anyhow::Result<()> {
        match self {
            Self::Stub { .. } => {
                error!("{:?}", err);
                Ok(())
            }
            Self::Full { id, tx } => tx
                .send(BrokerMessage::Error { id: *id, err })
                .with_context(|| "failed to send broker message"),
        }
    }
    pub fn info(&self, msg: String) -> anyhow::Result<()> {
        match self {
            Self::Stub { .. } => {
                info!("{}", msg);
                Ok(())
            }
            Self::Full { id, tx } => tx
                .send(BrokerMessage::Info { id: *id, msg })
                .with_context(|| "failed to send broker message"),
        }
    }
    pub fn black_box_stats(&self, stats: BlackBoxStats) -> anyhow::Result<()> {
        match self {
            Self::Stub { start } => Ok(info!("{}", stats.display(start))),
            Self::Full { id, tx } => tx
                .send(BrokerMessage::BlackBoxStats { id: *id, stats })
                .with_context(|| "failed to send broker message"),
        }
    }
    pub fn grey_box_stats(&self, stats: GreyBoxStats) -> anyhow::Result<()> {
        match self {
            Self::Stub { start } => Ok(info!("{}", stats.display(start))),
            Self::Full { id, tx } => tx
                .send(BrokerMessage::GreyBoxStats { id: *id, stats })
                .with_context(|| "failed to send broker message"),
        }
    }
    pub fn id(&self) -> u8 {
        match self {
            Self::Stub { .. } => 0,
            Self::Full { id, .. } => *id,
        }
    }
}
