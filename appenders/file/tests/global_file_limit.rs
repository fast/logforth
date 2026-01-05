// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::thread;
use std::time::Duration;

use logforth_append_file::FileBuilder;
use logforth_core::Append;
use logforth_core::record::Record;
use tempfile::TempDir;

#[test]
fn test_global_file_count_limit() {
    // Create a temporary directory for our test
    let temp_dir = TempDir::new().expect("failed to create a temporary directory");
    let max_files = 10; // Small number for testing
    let max_size = 100; // Small size to force rotation

    // Create our writer with hourly rotation
    let writer = FileBuilder::new(temp_dir.path(), "test_prefix")
        .rollover_hourly()
        .rollover_size(NonZeroUsize::new(max_size).unwrap())
        .max_log_files(NonZeroUsize::new(max_files).unwrap())
        .build()
        .unwrap();

    println!("Starting test_global_file_count_limit");

    // Write enough data to create multiple files
    for i in 0..50 {
        writer
            .append(
                &Record::builder()
                    .payload(format_args!("Log entry {}: {}\n", i, "A".repeat(50)))
                    .build(),
                &[],
            )
            .unwrap();
        writer.flush().unwrap();
    }

    // Count the total number of files with our prefix
    let files = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let filename = entry.file_name().to_str()?.to_string();
            if filename.starts_with("test_prefix") {
                Some(filename)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    println!("Found {} files: {:?}", files.len(), files);

    // Assert that the total number of files is limited to max_files
    assert!(
        files.len() <= max_files,
        "Expected at most {} files, but found {}: {:?}",
        max_files,
        files.len(),
        files
    );

    println!("Test passed! File count is limited to {}", max_files);
}

// This test case simulates the issue described: many log files exist across multiple dates,
// but the total count is not properly limited
#[test]
fn test_file_limit_across_multiple_dates() {
    // Create a temporary directory
    let temp_dir = TempDir::new().expect("failed to create a temporary directory");
    let max_files = 10; // Set maximum file count to 10
    let max_size = 50; // Small file size to trigger rotation quickly

    // Create multiple log writers, each with a different filename pattern (simulating different
    // dates)
    println!("Creating multiple log files with different date patterns");

    // First batch - simulate March 18
    create_logs(
        temp_dir.path(),
        max_files,
        max_size,
        "databend-query-default",
        15,
    );

    // Second batch - simulate March 19
    create_logs(
        temp_dir.path(),
        max_files,
        max_size,
        "databend-query-default",
        20,
    );

    // Third batch - simulate March 20
    create_logs(
        temp_dir.path(),
        max_files,
        max_size,
        "databend-query-default",
        10,
    );

    // Count all log files
    let files = count_log_files(temp_dir.path(), "databend-query-default");

    println!("Total files across all dates: {}", files.len());
    println!("Files: {:?}", files);

    // Verify if the total number of files exceeds the limit
    // Note: If this assertion fails, it means we've reproduced the issue - file count is not
    // properly limited
    assert!(
        files.len() <= max_files,
        "Expected at most {} files, but found {} files across multiple dates",
        max_files,
        files.len()
    );
}

// Create a specified number of log files
fn create_logs(dir: &Path, max_files: usize, max_size: usize, filename: &str, count: usize) {
    // Create a new log writer for each "date"
    let writer = FileBuilder::new(dir, filename)
        .rollover_hourly() // Use hourly rotation
        .rollover_size(NonZeroUsize::new(max_size).unwrap())
        .max_log_files(NonZeroUsize::new(max_files).unwrap())
        .build()
        .unwrap();

    println!("Creating logs with filename: {filename}");

    // Write enough data to create the specified number of files
    for i in 0..count * 5 {
        // Each file needs about 5 writes to rotate
        writer
            .append(
                &Record::builder()
                    .payload(format_args!(
                        "Prefix {}, Log {}: {}\n",
                        filename,
                        i,
                        "X".repeat(20)
                    ))
                    .build(),
                &[],
            )
            .unwrap();
        writer.flush().unwrap();

        // Brief pause to ensure the file system has time to process
        thread::sleep(Duration::from_millis(10));
    }
}

// Count log files with the specified prefix
fn count_log_files(dir: &Path, prefix: &str) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let filename = entry.file_name().to_str()?.to_string();
            if filename.starts_with(prefix) {
                Some(filename)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
