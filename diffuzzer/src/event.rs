use std::{
    io::Write,
    os::unix::net::UnixStream,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
};

use anyhow::{Context, bail};
use log::debug;
use serde::Deserialize;
use serde_json::{Deserializer, Value};

pub struct EventHandler {
    rx: Receiver<()>,
}

#[derive(Debug, Deserialize)]
struct ReturnMessage {
    #[serde(rename = "return")]
    _ret: Value,
}

impl EventHandler {
    pub fn create(socket_path: String) -> anyhow::Result<Self> {
        debug!("create event handler");
        let mut stream = UnixStream::connect(&socket_path)
            .with_context(|| format!("failed to connect to unix socket at '{}'", &socket_path))?;
        let mut de = Deserializer::from_reader(stream.try_clone()?);
        debug!("read greeting message:");
        let value =
            Value::deserialize(&mut de).with_context(|| "failed to deserialize response")?;
        debug!("{}", value);
        stream.write_all(b"{ \"execute\": \"qmp_capabilities\" }")?;
        debug!("read response (deserialized):");
        let return_msg = ReturnMessage::deserialize(&mut de)
            .with_context(|| "failed to deserialize return message")?;
        debug!("{:?}", return_msg);

        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

        thread::spawn(move || {
            loop {
                let value = Value::deserialize(&mut de)
                    .with_context(|| "failed to deserialize response")
                    .unwrap();
                debug!("received QMP message:\n{}", value);
                match value {
                    Value::Object(map) => {
                        if map.contains_key("event") {
                            tx.send(()).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { rx })
    }

    pub fn panicked(&mut self) -> anyhow::Result<bool> {
        let mut panicked = false;
        loop {
            match self.rx.try_recv() {
                Ok(()) => panicked = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => bail!("event channel disconnected"),
            }
        }
        Ok(panicked)
    }

    pub fn clear(&mut self) -> anyhow::Result<()> {
        loop {
            match self.rx.try_recv() {
                Ok(()) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => bail!("event channel disconnected"),
            }
        }
        Ok(())
    }
}
