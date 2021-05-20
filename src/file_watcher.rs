use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

pub fn watch(file: impl AsRef<Path>, tx: Sender<()>) {
    let (notif_tx, notif_rx) = channel();
    let mut watcher = watcher(notif_tx, Duration::from_millis(100)).unwrap();
    let cannon = file
        .as_ref()
        .canonicalize()
        .expect("Failed to canonicalize");

    let parent = cannon.parent().expect("File has no parent");

    watcher.watch(parent, RecursiveMode::NonRecursive).unwrap();

    loop {
        match notif_rx.recv() {
            Ok(DebouncedEvent::Write(b))
            | Ok(DebouncedEvent::Create(b))
            | Ok(DebouncedEvent::NoticeWrite(b)) => {
                if b.extension() == Some(std::ffi::OsStr::new("lua")) {
                    tx.send(()).expect("Watcher failed to send");
                }
            }
            Err(e) => println!("watch error: {:?}", e),
            _ => (),
        }
    }
}
