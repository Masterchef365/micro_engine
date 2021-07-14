use anyhow::{Context, Result};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

pub fn watch(file: impl AsRef<Path>, tx: Sender<PathBuf>) {
    if let Err(e) = watch_internal(file, tx) {
        println!("Watcher crashed! Reason: {}", e);
    }
}

fn watch_internal(file: impl AsRef<Path>, tx: Sender<PathBuf>) -> Result<()> {
    let (notif_tx, notif_rx) = channel();
    let mut watcher = watcher(notif_tx, Duration::from_millis(100))?;
    let cannon = file
        .as_ref()
        .canonicalize()
        .with_context(|| format!("Path \"{:?}\" to canonicalize", file.as_ref()))?;

    let parent = cannon
        .parent()
        .with_context(|| format!("Path \"{:?}\" has no parent folder", file.as_ref()))?;

    watcher.watch(parent, RecursiveMode::NonRecursive)?;

    loop {
        match notif_rx.recv() {
            Ok(DebouncedEvent::Write(b))
            | Ok(DebouncedEvent::Create(b))
            | Ok(DebouncedEvent::NoticeWrite(b)) => {
                tx.send(b)?;
            }
            Err(e) => println!("watch error: {:?}", e),
            _ => (),
        }
    }
}
