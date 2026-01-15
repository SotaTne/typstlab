//! Counter helper WITH file locking
//!
//! Usage: counter_child_locked <counter_path> <iterations>
//!
//! Performs read-modify-write operations on a counter file with locking.
//! This should prevent lost updates and preserve all increments.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use typstlab_core::lock::acquire_lock;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: counter_child_locked <counter_path> <iterations>");
        std::process::exit(1);
    }

    let counter_path = PathBuf::from(&args[1]);
    let iterations: usize = args[2].parse().expect("iterations must be a number");

    let lock_path = counter_path.with_extension("lock");

    for _ in 0..iterations {
        // Acquire lock (blocks until available)
        let _guard = acquire_lock(&lock_path, Duration::from_secs(10), "counter update")
            .expect("Failed to acquire lock");

        // Read current value
        let content = fs::read_to_string(&counter_path).expect("Failed to read counter file");
        let value: u32 = content
            .trim()
            .parse()
            .expect("Counter file should contain a number");

        // Increment (sleep to increase contention)
        std::thread::sleep(std::time::Duration::from_micros(10));
        let new_value = value + 1;

        // Write back (protected by lock)
        fs::write(&counter_path, new_value.to_string()).expect("Failed to write counter file");

        // Lock auto-released via Drop
    }

    println!("Counter child completed {} iterations", iterations);
}
