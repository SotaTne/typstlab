//! Helper binary for counter-based lost update testing
//!
//! Usage: counter_child <counter_path> <iterations>
//!
//! Performs read-modify-write operations on a counter file.
//! Without locking: Lost updates occur
//! With locking: All updates preserved

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: counter_child <counter_path> <iterations>");
        std::process::exit(1);
    }

    let counter_path = PathBuf::from(&args[1]);
    let iterations: usize = args[2].parse().expect("iterations must be a number");

    for _ in 0..iterations {
        // Read current value (with retry for race conditions)
        let value: u32 = match fs::read_to_string(&counter_path) {
            Ok(content) => content.trim().parse().unwrap_or(0),
            Err(_) => 0, // File might be temporarily unavailable
        };

        // Increment (sleep to increase contention)
        std::thread::sleep(std::time::Duration::from_micros(50));
        let new_value = value + 1;

        // Write back (NO LOCKING - intentional race condition)
        // May fail due to concurrent writes, but that's expected
        let _ = fs::write(&counter_path, new_value.to_string());
    }

    println!("Counter child completed {} iterations", iterations);
}
