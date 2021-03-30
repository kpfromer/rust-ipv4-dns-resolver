use anyhow::Result;
use chrono::Utc;
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use structopt::StructOpt;

pub const ARRAY_SIZE: usize = 10;
pub const MAX_INPUT_FILES: usize = 100;
pub const MAX_REQUESTER_THREADS: usize = 10;
pub const MAX_RESOLVER_THREADS: usize = 10;
// Not need for rust:
// pub const MAX_NAME_LENGTH: usize = 255;
// pub const MAX_IP_LENGTH: usize = 46;

// println that is thread safe
#[macro_use]
extern crate std;
macro_rules! threadprintln {
    ($dst:expr) => {
        use std::io::Write;
        let stdout = std::io::stdout();
        // Writeln macro
        writeln!(&mut stdout.lock(), $dst);
    };
    ($dst:expr, $($arg:tt)*) => {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        // Writeln macro
        writeln!(&mut stdout, $dst, $($arg)*);
    };
}

mod consumer;
mod producer;
mod shared_buffer;

use shared_buffer::SharedBuffer;

fn in_range(start: usize, end: usize) -> Box<dyn Fn(String) -> std::result::Result<(), String>> {
    Box::new(move |string| match string.parse::<usize>() {
        Ok(size) if (start..=end).contains(&size) => Ok(()),
        _ => Err(format!("Not in range [{}, {}]", start, end)),
    })
}

#[derive(Debug, StructOpt)]
#[structopt()]
struct Cli {
    #[structopt(validator = in_range(1, MAX_REQUESTER_THREADS))]
    num_requester: i32,

    #[structopt(validator = in_range(1, MAX_RESOLVER_THREADS))]
    num_resolver: i32,

    /// Input file
    #[structopt(parse(from_os_str))]
    requester_log_file: PathBuf,

    #[structopt(parse(from_os_str))]
    resolver_log_file: PathBuf,

    #[structopt(parse(from_os_str), required = true, max_values = MAX_INPUT_FILES as u64)]
    output: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let Cli {
        num_requester,
        num_resolver,
        requester_log_file,
        resolver_log_file,
        output,
    } = Cli::from_args();

    let input_files = Arc::new(Mutex::new(output));
    let shared = Arc::new(SharedBuffer::new());
    let serviced_writer = Arc::new(Mutex::new(File::create(requester_log_file)?));
    let resolved_writer = Arc::new(Mutex::new(File::create(resolver_log_file)?));

    let start_time = Utc::now().time();

    let requester_handles: Vec<JoinHandle<_>> = (0..num_requester)
        .into_iter()
        .map(|_| {
            let input_files = Arc::clone(&input_files);
            let shared = Arc::clone(&shared);
            let writer = Arc::clone(&serviced_writer);

            thread::spawn(move || {
                producer::producer(input_files, shared, writer).unwrap();
            })
        })
        .collect();

    let resolver_handles: Vec<JoinHandle<_>> = (0..num_resolver)
        .into_iter()
        .map(|_| {
            let shared = Arc::clone(&shared);
            let writer = Arc::clone(&resolved_writer);

            thread::spawn(move || {
                consumer::consumer(shared, writer).unwrap();
            })
        })
        .collect();

    for handle in requester_handles {
        handle.join().unwrap();
    }

    for handle in resolver_handles {
        handle.join().unwrap();
    }

    let end_time = Utc::now().time();
    let diff = end_time - start_time;

    println!(
        "Time to run: {} seconds",
        diff.num_milliseconds() as f64 / 1000.0
    );

    Ok(())
}
