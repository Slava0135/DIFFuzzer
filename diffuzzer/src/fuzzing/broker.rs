use std::{sync::mpsc::Sender, time::Instant};

use anyhow::Context;
use log::{error, info};

pub enum BrokerMessage {
    Error {
        id: u8,
        err: anyhow::Error,
    },
    Stats {
        id: u8,
        crashes: u64,
        executions: u64,
    },
    Info {
        id: u8,
        msg: String,
    },
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
    pub fn stats(&self, crashes: u64, executions: u64) -> anyhow::Result<()> {
        match self {
            Self::Stub { start } => Ok(info!("{}", stats_string(start, crashes, executions))),
            Self::Full { id, tx } => tx
                .send(BrokerMessage::Stats {
                    id: *id,
                    executions,
                    crashes,
                })
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

pub fn stats_string(start: &Instant, crashes: u64, executions: u64) -> String {
    let secs = start.elapsed().as_secs();
    format!(
        "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
        crashes,
        executions,
        (executions as f64) / (secs as f64),
        secs / (60 * 60),
        (secs / (60)) % 60,
        secs % 60,
    )
}
