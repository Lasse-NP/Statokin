use std::io;
use std::fs;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use inotify::{
    Inotify,
    WatchDescriptor,
    WatchMask,
    EventMask
};


pub struct Watcher {
    file_count: u32,
    inotify: Inotify,
    watch_map: HashMap<WatchDescriptor, PathBuf>,
    top_path: PathBuf,
}

impl Watcher {
    pub fn new(path: String) -> Result<Self, io::Error> {
        println!("Watcher Initializing...");
        let path = PathBuf::from(path);
        println!("Watcher Path Granted: {}", path.display());
        let mut noti = Inotify::init()?;
        let mut map = HashMap::new();
        let file_count = count_files(&path)?;
        if file_count > 100000 {
            println!("Chosen path is highly populated with files: {} files", file_count);
            println!("Proceed anyway? (y/N)");
            let stdin = io::stdin();
            let mut input = String::new();
            stdin.lock().read_line(&mut input).expect("Failed to read input");
            match input.trim() {
                "y" | "Y" => {},
                _ => {
                    println!("Watcher Aborted.");
                    std::process::exit(0);
                },
            }
        } else {
            println!("Total file count: {} files", file_count);
            println!("Proceeding...");
        }
        mark_dirs(&path, &mut map, &mut noti)?;
        println!("Watcher established at {} with {} entries.", path.display(), map.len());
        Ok(Watcher { file_count, inotify: noti, watch_map: map, top_path: path })
    }

    pub fn run(&mut self) {
        let mut buffer = [0u8; 4096];
        loop {
            let events = self.inotify.read_events_blocking(&mut buffer).expect("Failed to read events.");
            let map_ref = &self.watch_map;
            for event in events {
                if let Some(path) = map_ref.get(&event.wd) {
                    let name = event.name.as_deref().unwrap_or_default();
                    let action = mask_to_string(&event.mask);
                    let full_path = path.join(&name);
                    println!("File: {} | Operation: {}", full_path.display(), action)
                }
            }
        }
    }

    pub fn get_top_path(&self) -> &PathBuf {
        &self.top_path
    }

    pub fn get_watch_map(&self) -> &HashMap<WatchDescriptor, PathBuf> {
        &self.watch_map
    }
}

fn count_files(dir: &Path) -> io::Result<u32> {
    let mut count = 0;
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            count += 1;
            if path.is_dir() {
                count += 1;
                count += count_files(&path)?;
            }
        }
    }
    Ok(count)
}

fn mark_dirs(dir: &Path, map: &mut HashMap<WatchDescriptor, PathBuf>, inotify: &mut Inotify) -> io::Result<()> {
    if dir.is_dir() {
        let watch_entry = inotify.watches().add(dir, WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE | WatchMask::MOVED_TO | WatchMask::MOVED_FROM)?;
        map.insert(watch_entry.clone(), dir.to_path_buf());
        println!("Watcher Entry: {}, Path to Watcher: {}", &watch_entry.get_watch_descriptor_id(), &dir.display());
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                mark_dirs(&path, map, inotify)?;
            }
        }
    }
    Ok(())
}

fn mask_to_string(mask: &EventMask) -> &str {
    if mask.contains(EventMask::CREATE) {
        "File Created"
    } else if mask.contains(EventMask::MODIFY) {
        "File Modified"
    } else if mask.contains(EventMask::DELETE) {
        "File Deleted"
    } else if mask.contains(EventMask::MOVED_FROM) {
        "File Moved From"
    } else if mask.contains(EventMask::MOVED_TO) {
        "File Moved To"
    } else {
        "Unknown Action"
    }
}