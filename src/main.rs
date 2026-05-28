mod watch;
use std::path::Path;

use watch::Watcher;
use watch::count_files;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short = 'P', long = "path")]
    path: String,

    #[arg(short = 'C', long = "count")]
    count_only: bool
}

fn main() {
    let args = Args::parse();
    if args.count_only {
        let count = count_files(Path::new(&args.path)).expect("Failed to find file count.");
        println!("There exists {} files in {}", count, args.path);
    } else {
        let mut watcher = Watcher::new(args.path).expect("Failed to initialize Watcher");
        watcher.run();
    }
}
