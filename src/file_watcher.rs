use notify::{Event, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::Path;
use std::sync::mpsc::Sender;

pub fn watch(file: impl AsRef<Path>, tx: Sender<()>) {
    let path_buf = file.as_ref().to_path_buf();
    let mut watcher: RecommendedWatcher =
        Watcher::new_immediate(move |res: Result<Event>| match res {
            Ok(event) => {
                if event.paths.contains(&path_buf) {
                    tx.send(()).expect("Watcher failed to send");
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })
        .expect("Failed to init watcher");
    let cannon = file
        .as_ref()
        .canonicalize()
        .expect("Failed to canonicalize");
    let parent = cannon.parent().expect("File has no parent");
    println!("{:?}", parent);
    watcher
        .watch(parent, RecursiveMode::NonRecursive)
        .expect("Failed to start watcher");
}
