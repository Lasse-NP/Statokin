mod watch;
use watch::Watcher;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short = 'P')]
    path: String,
}

fn main() {
    let args = Args::parse();
    let mut watcher = Watcher::new(args.path).expect("Failed to initialize Watcher");
    watcher.run();
}
