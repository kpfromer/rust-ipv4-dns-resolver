use crate::SharedBuffer;
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn get_files_in_directory<P>(directory: P) -> Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    Ok(fs::read_dir(directory)?
        .filter_map(|entry| entry.map(|file| file.path()).ok())
        .collect::<Vec<_>>())
}

pub fn producer(
    files: Arc<Mutex<Vec<PathBuf>>>,
    shared: Arc<SharedBuffer<String>>,
    log_file: Arc<Mutex<File>>,
) -> Result<u32> {
    let mut serviced: u32 = 0;

    // Say we are starting
    {
        let mut buffer = shared.buffer.lock().unwrap();
        buffer.running_producers += 1;
    }

    while let Some(mut file) = {
        let mut input_files = files.lock().unwrap();
        (*input_files).pop()
    } {
        file = if file.is_dir() {
            let mut files_under_dir = get_files_in_directory(file)?;
            let current_file = files_under_dir.pop();

            // If there are files then use the one on top and add the rest to the shared files vector
            if let Some(current_file) = current_file {
                // Loop through, and try to acquire lock and push file on shared files vector
                for new_file in files_under_dir {
                    let mut files = files.lock().unwrap();
                    files.push(new_file);
                }
                current_file
            } else {
                // Otherwise there are no files and we will continue looking in the shared files vector
                continue;
            }
        } else {
            file
        };

        serviced += 1;

        if let Ok(lines) = read_lines(file) {
            for line in lines {
                if let Ok(hostname) = line {
                    let hostname_copy = hostname.clone();
                    {
                        // Wait until buffer can produce and is not empty
                        let mut buffer = shared.buffer.lock().unwrap();
                        while (*buffer).is_full() {
                            buffer = shared.can_produce.wait(buffer).unwrap();
                        }

                        // println!("Pushed {}", hostname);
                        (*buffer)
                            .push(hostname)
                            .with_context(|| "Failed to push to buffer!")?;

                        shared.can_consume.notify_one();
                    }

                    let out = format!("{}\n", hostname_copy).into_bytes();
                    let mut log_file = log_file.lock().unwrap();
                    log_file.write_all(&out)?;
                }
            }
        }
    }

    // Say we are done
    let still_running = {
        let mut buffer = shared.buffer.lock().unwrap();
        buffer.running_producers -= 1;

        buffer.running_producers
    };

    // Tell the consumers that we are done producing
    if still_running == 0 {
        shared.can_consume.notify_all();
    }

    Ok(serviced)
}
