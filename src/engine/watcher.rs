use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result, Watcher as _};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct Watcher {
    dirty: Arc<AtomicBool>,
    modified_files: Arc<Mutex<Vec<std::path::PathBuf>>>,
}

impl Watcher {
    pub fn new<P: AsRef<Path>>(paths: &[P]) -> Result<Self> {
        let dirty = Arc::new(AtomicBool::new(false));
        let modified_files = Arc::new(Mutex::new(Vec::from_iter(
            paths.iter().map(|p| p.as_ref().to_path_buf()),
        )));
        let dirty_clone = dirty.clone();
        let modified_files_clone = modified_files.clone();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event>| {
                let event = result.unwrap();
                if event.kind.is_modify() {
                    let mut files = modified_files_clone.lock().unwrap();
                    for path in event.paths {
                        files.push(path);
                    }
                    dirty_clone.store(true, Ordering::SeqCst);
                }
            },
            Config::default().with_poll_interval(std::time::Duration::from_secs(1)),
        )?;

        for path in modified_files.lock().unwrap().iter() {
            watcher.watch(path, RecursiveMode::NonRecursive)?;
        }

        std::mem::forget(watcher);

        Ok(Watcher {
            dirty,
            modified_files,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::SeqCst)
    }

    pub fn take_modified_files(&self) -> Vec<PathBuf> {
        let mut files = self.modified_files.lock().unwrap();
        let taken_files = files.drain(..).collect();
        self.dirty.store(false, Ordering::SeqCst);
        taken_files
    }
}
