use std::io;
use std::fs;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::Instant;
use inotify::{
    Inotify,
    WatchDescriptor,
    WatchMask,
    EventMask
};


pub struct Watcher {
    file_map: HashMap<PathBuf, Vec<EventMask>>,
    inotify: Inotify,
    watch_map: HashMap<WatchDescriptor, PathBuf>,
}

impl Watcher {
    pub fn new(path: String) -> Result<Self, io::Error> {
        println!("Watcher Initializing...");
        let path = PathBuf::from(path);
        println!("Watcher Path Granted: {}", path.display());

        let mut noti = Inotify::init()?;
        let mut watch_map = HashMap::new();
        let mut file_map = HashMap::new();

        let file_count = map_files(&path, &mut file_map)?;

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

        mark_dirs(&path, &mut watch_map, &mut noti)?;

        if watch_map.len() == 1 {
            println!("Watcher established at {} with {} directory. Currently watching: {} files.", path.display(), watch_map.len(), file_count);
        } else {
            println!("Watcher established at {} with {} directories. Currently watching: {} files.", path.display(), watch_map.len(), file_count);
        }

        Ok(Watcher { file_map, inotify: noti, watch_map: watch_map })
    }

    pub fn run(&mut self) {
        let mut buffer = [0u8; 4096];
        let mut pending_moves: HashMap<u32, (PathBuf, Instant)> = HashMap::new();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            let expired: Vec<u32> = pending_moves.iter().filter(|(_, value)| value.1.elapsed().as_secs() > 3).map(|(cookie, _)| *cookie).collect();
            for cookie in expired {
                if let Some((old_path, _)) = pending_moves.remove(&cookie) {
                    self.file_map.remove(&old_path);
                    println!("File: {} -> Unknown | Operation: Exited to Outside Scope", old_path.display())
                }
            }

            let events = match self.inotify.read_events(&mut buffer) {
                Ok(events) => events,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("Failed to read events: {}", e)
            };

            for event in events {
                if let Some(path) = self.watch_map.get(&event.wd) {
                    let name = event.name.as_deref().unwrap_or_default();
                    let action = mask_to_string(&event.mask);
                    let full_path = path.join(&name);
                    if event.mask.contains(EventMask::MOVED_FROM) {
                        pending_moves.insert(event.cookie, (full_path.clone(), Instant::now()));
                    } else if event.mask.contains(EventMask::MOVED_TO) {
                        if let Some(value) = pending_moves.remove(&event.cookie) {
                            let old_path = value.0;
                            let vector_data = self.file_map.remove(&old_path).unwrap_or_default();
                            self.file_map.insert(full_path.clone(), vector_data);
                            println!("File: {} -> {} | Operation: Moved To New Location", old_path.display(), full_path.display())
                        } else {
                            self.file_map.entry(full_path.clone()).or_insert_with(Vec::new).push(event.mask);
                            println!("File: Unknown -> {} | Operation: Entered From Outside Scope", full_path.display())
                        }
                    } else {
                        self.file_map.entry(full_path.clone()).or_insert_with(Vec::new).push(event.mask);
                        println!("File: {} | Operation: {}", full_path.display(), action)
                    }
                }
            }
        }
    }
}

pub fn count_files(dir: &Path) -> io::Result<u32> {
    let mut count = 0;
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            count += 1;
            if path.is_dir() {
                count += count_files(&path)?;
            }
        }
    }
    Ok(count)
}

fn map_files(dir: &Path, map: &mut HashMap<PathBuf, Vec<EventMask>>) -> io::Result<u32> {
    let mut count = 0;
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            count += 1;
            map.insert(path.clone(), Vec::new());
            if path.is_dir() {
                count += map_files(&path, map)?;
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