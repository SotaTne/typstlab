//! Helper binary that acquires lock, writes marker, holds, releases
//!
//! Usage: lock_holder <lock_path> <marker_path> <process_id>
//!
//! This binary is used to test cross-process exclusive locking.
//! It acquires a lock, writes a marker to a file, holds the lock briefly,
//! then releases it. Multiple processes running this should execute sequentially.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use typstlab_core::lock::acquire_lock;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: lock_holder <lock_path> <marker_path> <process_id>");
        std::process::exit(1);
    }

    let lock_path = PathBuf::from(&args[1]);
    let marker_path = PathBuf::from(&args[2]);
    let process_id = &args[3];

    // Acquire lock (blocks until available)
    let _guard = acquire_lock(
        &lock_path,
        Duration::from_secs(30),
        &format!("process {}", process_id),
    )
    .expect("Failed to acquire lock");

    // Write marker with process ID
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(marker_path)
        .expect("Failed to open marker file");

    writeln!(file, "process_{} acquired lock", process_id).expect("Failed to write marker");

    // Hold lock for a bit to ensure sequential execution
    std::thread::sleep(Duration::from_millis(100));

    // Lock auto-released via Drop
    println!("Process {} completed", process_id);
}
