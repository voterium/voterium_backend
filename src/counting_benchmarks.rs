// Various implementations of vote countings.
// The following benchmark is based on 10M votes from 10M voters (i.e. no recast votes)
//
// count_votes_1 - total 4.753971375s  -  open and buffer file: 41.042µs, parse lines and collect votes 1.485171167s, process votes into latest_votes 2.149584666s, count votes 1.119173834s
// count_votes_2 - total 8.007304333s  -  open and buffer file: 39.005541ms, parse lines and collect votes 21.833µs, process votes into latest_votes 6.828549s, count votes 1.139727167s
// count_votes_3 - total 3.485497625s  -  open and buffer file: 31.875µs, process votes to latest_votes 2.309327167s, count votes 1.176135583s
// count_votes_4 - total 2.929065375s  -  open and buffer file: 30.209µs, count lines 77.372916ms, process votes to latest_votes 1.611441125s, count votes 1.240220041s
// count_votes_5 - total 1.609642458s  -  open and buffer file: 49.75µs, process votes to latest_votes 1.37704375s, count votes 232.547958ms
// count_votes_6 - total 1.591099416s  -  open and buffer file: 33.829084ms, process votes to latest_votes 1.304531833s, count votes 252.732416ms
// count_votes_7 - total 3.426512875s  -  open and buffer file: 21.917µs, process votes to latest_votes 2.274333458s, count votes 1.152156375s
// count_votes_8 - total 1.4050165s  -  open and buffer file: 109.03575ms, process votes to latest_votes 1.074815084s, count votes 221.163125ms
// count_votes_9 - total 3.44481675s  -  open and buffer file: 26.75µs, process votes to latest_votes 2.279224875s, count votes 1.165563s
//
// and the next one is based on ~10M votes from 2100 voters (i.e. many recast votes)
//
// count_votes_1 - total 3.347919417s  -  open and buffer file: 60.875µs, parse lines and collect votes 1.949741792s, process votes into latest_votes 1.397493041s, count votes 623.375µs
// count_votes_2 - total 5.848337417s  -  open and buffer file: 55.7855ms, parse lines and collect votes 17.417µs, process votes into latest_votes 5.791894542s, count votes 639.541µs
// count_votes_3 - total 1.318625667s  -  open and buffer file: 31.708µs, process votes to latest_votes 1.317990208s, count votes 603.458µs
// count_votes_4 - total 1.404618708s  -  open and buffer file: 34.708µs, count lines 97.348042ms, process votes to latest_votes 1.304892542s, count votes 2.343167ms
// count_votes_5 - total 382.446583ms  -  open and buffer file: 50.125µs, process votes to latest_votes 380.992ms, count votes 1.403542ms
// count_votes_6 - total 428.765625ms  -  open and buffer file: 47.914584ms, process votes to latest_votes 380.265416ms, count votes 585µs
// count_votes_7 - total 1.213581s  -  open and buffer file: 37.125µs, process votes to latest_votes 1.212899959s, count votes 643.416µs
// count_votes_8 - total 610.525166ms  -  open and buffer file: 168.750958ms, process votes to latest_votes 439.975791ms, count votes 1.797167ms
// count_votes_9 - total 1.314916s  -  open and buffer file: 38.125µs, process votes to latest_votes 1.314256125s, count votes 621.25µs

use crate::models::{CLVote, Choice, VoteCount};
use csv::ReaderBuilder;
use log::{info, warn};
use memchr::{memchr, memchr2_iter, memchr_iter, memrchr};
use memmap2::Mmap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHasher};
use std::collections::HashMap;
use std::fs::File;
use std::hash::BuildHasherDefault;
use std::io::Seek;
use std::io::{BufRead, BufReader, Read};
use std::time::Instant;

pub fn count_votes_1() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Read the CL file
    let start_read = Instant::now();
    let file = File::open("cl.csv")?;
    let reader = BufReader::new(file);
    let duration_read = start_read.elapsed();

    let mut votes: Vec<CLVote> = Vec::new();

    // Parse lines
    let start_parse = Instant::now();
    for line in reader.lines() {
        let line = line?;

        let parts: Vec<&str> = line.trim().split(',').collect();
        if parts.len() != 3 {
            continue; // Skip malformed lines
        }

        let timestamp = parts[1]
            .parse::<i64>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let vote = CLVote {
            user_id_hash: parts[0].to_string(),
            timestamp,
            choice: parts[2].to_string(),
        };

        votes.push(vote);
    }
    let duration_parse = start_parse.elapsed();

    // Build a map of user_id_hash to their latest vote
    let start_process = Instant::now();
    let mut latest_votes: HashMap<String, CLVote> = HashMap::new();
    for vote in votes {
        latest_votes
            .entry(vote.user_id_hash.clone())
            .and_modify(|existing_vote| {
                if vote.timestamp > existing_vote.timestamp {
                    *existing_vote = vote.clone();
                }
            })
            .or_insert(vote);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for vote in latest_votes.values() {
        *counts.entry(vote.choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount { choice, count })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_1 - total {:?}  -  open and buffer file: {:?}, parse lines and collect votes {:?}, process votes into latest_votes {:?}, count votes {:?}",
            duration_total, duration_read, duration_parse, duration_process, duration_count);

    Ok(vote_counts)
}

pub fn count_votes_2() -> Result<Vec<VoteCount>, Box<dyn std::error::Error>> {
    use std::time::Instant;

    let start_total = Instant::now();

    // Read the entire file into a String to own the data
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let duration_read = start_read.elapsed();

    // Create a CSV reader from the string
    let start_parse = Instant::now();
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .trim(csv::Trim::All)
        .from_reader(contents.as_bytes());
    let duration_parse_setup = start_parse.elapsed();

    // Process records
    let start_process = Instant::now();
    let mut latest_votes: HashMap<String, String> = HashMap::new();

    for result in rdr.records() {
        let record = result?;

        if record.len() != 3 {
            continue; // Skip malformed lines
        }

        let user_id_hash = record.get(0).unwrap().to_string();
        // We can ignore timestamp since votes are sorted
        let choice = record.get(2).unwrap().to_string();

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount { choice, count })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_2 - total {:?}  -  open and buffer file: {:?}, parse lines and collect votes {:?}, process votes into latest_votes {:?}, count votes {:?}",
             duration_total, duration_read, duration_parse_setup, duration_process, duration_count);

    Ok(vote_counts)
}

pub fn count_votes_3() -> Result<Vec<VoteCount>, std::io::Error> {
    use std::time::Instant;

    let start_total = Instant::now();

    // Open the file and create a buffered reader
    let start_read = Instant::now();
    let file = File::open("cl.csv")?;
    let mut reader = BufReader::new(file);
    let duration_read = start_read.elapsed();

    // Since we know the file size, we can preallocate buffers
    let start_process = Instant::now();

    // Prepare buffers for lines and parts to reduce allocations
    let mut line = String::new();
    let mut latest_votes: HashMap<String, String> = HashMap::new();

    while reader.read_line(&mut line)? > 0 {
        let line_trimmed = line.trim();

        // Find indices of the commas
        let mut parts_iter = line_trimmed.splitn(3, ',');
        let user_id_hash = match parts_iter.next() {
            Some(s) => s,
            None => {
                line.clear();
                continue; // Skip malformed lines
            }
        };

        // Skip the timestamp since data is sorted
        parts_iter.next();

        let choice = match parts_iter.next() {
            Some(s) => s,
            None => {
                line.clear();
                continue; // Skip malformed lines
            }
        };

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash.to_string(), choice.to_string());

        line.clear(); // Clear the line buffer for the next read
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount { choice, count })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_3 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
             duration_total, duration_read, duration_process, duration_count);

    Ok(vote_counts)
}

fn count_lines_fast(reader: &mut BufReader<File>) -> Result<usize, std::io::Error> {
    let mut buffer = [0; 8192];
    let mut line_count = 0;

    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        line_count += buffer[..bytes_read].iter().filter(|&&c| c == b'\n').count();
    }
    reader.rewind()?;
    Ok(line_count)
}

pub fn count_votes_4() -> Result<Vec<VoteCount>, std::io::Error> {
    use std::time::Instant;

    let start_total = Instant::now();

    // Open the file and create a buffered reader
    let start_read = Instant::now();
    let file = File::open("cl.csv")?;
    let mut reader = BufReader::new(file);
    let duration_read = start_read.elapsed();

    // Count the number of lines in the file
    let start_count_lines = Instant::now();
    let line_count = count_lines_fast(&mut reader)?;
    let duration_count_lines = start_count_lines.elapsed();

    let start_process = Instant::now();

    // Prepare buffers for lines and parts to reduce allocations
    let mut line = String::new();
    let mut latest_votes: FxHashMap<String, String> = FxHashMap::default();
    latest_votes.reserve(line_count);

    while reader.read_line(&mut line)? > 0 {
        let line_trimmed = line.trim();

        // Find indices of the commas
        let mut parts_iter = line_trimmed.splitn(3, ',');
        let user_id_hash = match parts_iter.next() {
            Some(s) => s,
            None => {
                line.clear();
                continue; // Skip malformed lines
            }
        };

        // Skip the timestamp since data is sorted
        parts_iter.next();

        let choice = match parts_iter.next() {
            Some(s) => s,
            None => {
                line.clear();
                continue; // Skip malformed lines
            }
        };

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash.to_string(), choice.to_string());

        line.clear(); // Clear the line buffer for the next read
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount { choice, count })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_4 - total {:?}  -  open and buffer file: {:?}, count lines {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_count_lines, duration_process, duration_count
    );

    Ok(vote_counts)
}

pub fn count_votes_5() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and memory-map it
    let start_read = Instant::now();
    let file = File::open("cl.csv")?;
    let mmap = unsafe { Mmap::map(&file)? };
    let data = &*mmap;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<&[u8], u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_5 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}

pub fn count_votes_6() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<&[u8], u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    println!("count_votes_6 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_7() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<Vec<u8>, Vec<u8>> = FxHashMap::default();

    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer size
    let mut buf = vec![0u8; BUFFER_SIZE];
    let mut bytes_in_buffer = 0;

    loop {
        // Read data into buf[bytes_in_buffer..]
        let bytes_read = file.read(&mut buf[bytes_in_buffer..])?;
        if bytes_read == 0 {
            break; // EOF reached
        }
        bytes_in_buffer += bytes_read;
        let buf_slice = &buf[..bytes_in_buffer];

        // Find the last newline in buf_slice
        let last_newline_index = match memrchr(b'\n', buf_slice) {
            Some(index) => index,
            None => {
                // No newline found in buffer
                if bytes_in_buffer == buf.len() {
                    // Buffer full but no newline, likely a very long line
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Line too long or no newline found in buffer",
                    ));
                }
                continue; // Read more data to find a newline
            }
        };

        // Process complete lines in buf_slice[..=last_newline_index]
        let lines = &buf_slice[..=last_newline_index]; // Include the newline

        let mut start = 0;
        while start < lines.len() {
            // Find the next newline
            let next_newline = memchr(b'\n', &lines[start..]).unwrap();
            let line = &lines[start..start + next_newline]; // Exclude the newline
            start += next_newline + 1; // Move past the newline

            // Split the line by commas
            let mut field_start = 0;
            let mut parts = Vec::with_capacity(3);
            for _ in 0..3 {
                match memchr(b',', &line[field_start..]) {
                    Some(pos) => {
                        parts.push(&line[field_start..field_start + pos]);
                        field_start += pos + 1;
                    }
                    None => {
                        parts.push(&line[field_start..]);
                        break;
                    }
                }
            }
            if parts.len() < 3 {
                // Skip malformed line
                continue;
            }

            let user_id_hash = parts[0].to_vec();
            // Skip timestamp (parts[1])
            let choice = parts[2].to_vec();

            // Overwrite the latest vote for the user
            latest_votes.insert(user_id_hash, choice);
        }

        // Copy any remaining bytes after last_newline_index to the beginning of buf
        let remaining = bytes_in_buffer - (last_newline_index + 1);
        buf.copy_within((last_newline_index + 1)..bytes_in_buffer, 0);
        bytes_in_buffer = remaining;
    }

    // After the loop, process any remaining data in buf[..bytes_in_buffer]
    if bytes_in_buffer > 0 {
        let line = &buf[..bytes_in_buffer];

        // Split the line by commas
        let mut field_start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[field_start..]) {
                Some(pos) => {
                    parts.push(&line[field_start..field_start + pos]);
                    field_start += pos + 1;
                }
                None => {
                    parts.push(&line[field_start..]);
                    break;
                }
            }
        }
        if parts.len() >= 3 {
            let user_id_hash = parts[0].to_vec();
            // Skip timestamp (parts[1])
            let choice = parts[2].to_vec();

            // Overwrite the latest vote for the user
            latest_votes.insert(user_id_hash, choice);
        }
    }

    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: String::from_utf8(choice).unwrap_or_default(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_7 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}

pub fn count_votes_8() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();

    let file = File::open("cl.csv")?;
    let mut reader = BufReader::new(file);
    let n_lines = count_lines_fast(&mut reader)?;

    let mut file = File::open("cl.csv")?;
    // let mut data = Vec::new();
    let mut data = Vec::with_capacity(n_lines * 64); // Assume average line length of 64 bytes
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
    latest_votes.reserve(n_lines);

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<&[u8], u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_8 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}

fn find_new_line_pos(bytes: &[u8]) -> Option<usize> {
    // In this case (position is not far enough),
    // naive version is faster than bstr (memchr)
    bytes.iter().rposition(|&b| b == b'\n')
}

pub fn count_votes_9() -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    // let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
    let mut latest_votes: FxHashMap<Vec<u8>, Vec<u8>> = FxHashMap::default();

    const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer size

    let mut buf = vec![0u8; BUFFER_SIZE];
    let mut bytes_not_processed = 0;

    while let Ok(n_bytes_read) = file.read(&mut buf[bytes_not_processed..]) {
        if n_bytes_read == 0 {
            break; // EOF reached
        }

        let actual_buf = &mut buf[..bytes_not_processed + n_bytes_read];
        let last_new_line_index = match find_new_line_pos(&actual_buf) {
            Some(index) => index,
            None => {
                warn!("No new line found in the read buffer");
                bytes_not_processed += n_bytes_read;
                if bytes_not_processed == buf.len() {
                    panic!("No new line found in the read buffer");
                }
                continue; // try again, maybe we next read will have a newline
            }
        };

        let chunk = &actual_buf[..last_new_line_index + 1];

        for line in chunk.split(|&b| b == b'\n') {
            let mut start = 0;
            let mut parts = Vec::with_capacity(3);
            for _ in 0..3 {
                match memchr(b',', &line[start..]) {
                    Some(pos) => {
                        parts.push(&line[start..start + pos]);
                        start += pos + 1;
                    }
                    None => {
                        parts.push(&line[start..]);
                        break;
                    }
                }
            }
            if parts.len() < 3 {
                // Skip malformed line
                bytes_not_processed = &actual_buf.len() - last_new_line_index - 1;
                continue;
            }

            let user_id_hash = parts[0].to_vec();
            // Skip timestamp (parts[1])
            let choice = parts[2].to_vec();

            // Overwrite the latest vote for the user
            latest_votes.insert(user_id_hash, choice);
        }

        actual_buf.copy_within(last_new_line_index + 1.., 0);
        // You cannot use bytes_not_processed = bytes_read - last_new_line_index
        // - 1; because the buffer will contain unprocessed bytes from the
        // previous iteration and the new line index will be calculated from the
        // start of the buffer
        bytes_not_processed = actual_buf.len() - last_new_line_index - 1;
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();
    let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
    for choice in latest_votes.values() {
        *counts.entry(choice.clone()).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: String::from_utf8(choice).unwrap_or_default(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_9 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_10(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: HashMap<&[u8], u32> = HashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_10 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_11(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_11 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}



pub fn count_votes_12(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let mut start = 0;
        let mut parts = Vec::with_capacity(3);
        for _ in 0..3 {
            match memchr(b',', &line[start..]) {
                Some(pos) => {
                    parts.push(&line[start..start + pos]);
                    start += pos + 1;
                }
                None => {
                    parts.push(&line[start..]);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = parts[0];
        // Skip timestamp (parts[1])
        let choice = parts[2];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    // let choice_keys: HashSet<&[u8]> = HashSet::from_iter(choices.iter().map(|choice| choice.key.as_bytes()));
    // for choice in latest_votes.iter().filter_map(|(key, value)| {
    //     if choice_keys.contains(value) {
    //         Some(value)
    //     } else {
    //         None
    //     }
    // }) {
    //     *counts.entry(*choice).or_insert(0) += 1;
    // }


    let mut choice_keys = choices.iter().map(|choice| choice.key.as_bytes()).collect::<Vec<_>>();
    for choice in latest_votes.iter().filter_map(|(key, value)| 
        if choice_keys.contains(value) {
            Some(value)
        } else {
            None
        }
    ) {
        *counts.entry(*choice).or_insert(0) += 1;
    }


    // let mut choice_keys = choices.iter().map(|choice| choice.key.as_bytes()).collect::<Vec<_>>();
    // choice_keys.sort_unstable();
    // for choice in latest_votes.iter().filter_map(|(key, value)| 
    //     choice_keys.binary_search(value).map(|_| value).ok()
    // ) {
    //     *counts.entry(*choice).or_insert(0) += 1;
    // }

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_12 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_13(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut i = 0;
    let data_len = data.len();

    while i < data_len {
        // Find the next newline
        let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
        // let next_newline = data[i..].iter().position(|&b| b == b'\n').unwrap_or(data_len - i);
        let line = &data[i..i + next_newline];
        i += next_newline + 1; // Move past the newline

        // Split the line by commas
        let commas: Vec<usize> = memchr_iter(b',', &line).collect();
        if commas.len() != 2 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = &line[0..commas[0]];
        let choice = &line[commas[1] + 1..];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }
    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_13 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}



pub fn count_votes_14(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut line_start = 0;
    for line_end in memchr_iter(b'\n', &data) {
        let line = &data[line_start..line_end];
        line_start = line_end + 1;

        // Split the line by commas
        let commas: Vec<usize> = memchr_iter(b',', &line).collect();
        if commas.len() != 2 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = &line[..commas[0]];
        let choice = &line[commas[1] + 1..];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }

    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_14 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_15(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    let mut line_start = 0;
    for line_end in memchr_iter(b'\n', &data) {
        let line = &data[line_start..line_end];
        line_start = line_end + 1;

        // Split the line by commas
        let mut comma_indices = [0; 2];
        let mut comma_count = 0;
        for (i, &byte) in line.iter().enumerate() {
            if byte == b',' {
                if comma_count < 2 {
                    comma_indices[comma_count] = i;
                    comma_count += 1;
                } else {
                    break;
                }
            }
        }
        if comma_count != 2 {
            // Skip malformed line
            continue;
        }

        let user_id_hash = &line[..comma_indices[0]];
        let choice = &line[comma_indices[1] + 1..];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
    }

    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_15 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


fn fast_split(data: &[u8], delimiter: u8) -> impl Iterator<Item = &[u8]> {
    // use memchr_iter
    memchr_iter(delimiter, data)
        .scan(0, |start, end| {
            let slice = &data[*start..end];
            *start = end + 1;
            Some(slice)
        })
}


pub fn count_votes_16(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let file_size = file.metadata()?.len() as usize;
    let mut data = Vec::with_capacity(file_size);
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

    for line in fast_split(&data, b'\n') {
        let mut commas = memchr_iter(b',', line);
        if let Some(c1) = commas.next() {
            if let Some(c2) = commas.next() {
                let user_id_hash = &line[..c1];
                let choice = &line[c2 + 1..];

                // Overwrite the latest vote for the user
                latest_votes.insert(user_id_hash, choice);
            }
        }
    }


    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_16 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}



pub fn count_votes_17(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let file_size = file.metadata()?.len() as usize;
    let mut data = Vec::with_capacity(file_size);
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
    
    
    let mut matches =  memchr2_iter(b'\n', b',', &data);    
    let mut prev_newline = 0;

    // // let mut c1 = 0;
    // let mut c2 = 0;
    // let mut newline = 0;


    // while let Some(c1) = matches.next() {
    //     c2 = matches.next().unwrap();
    //     newline = matches.next().unwrap();

    //     let user_id_hash = &data[prev_newline..c1];
    //     let choice = &data[c2 + 1..newline];

    //     // Overwrite the latest vote for the user
    //     latest_votes.insert(user_id_hash, choice);
    //     prev_newline = newline;
    // }

    let mut user_id_hash: &[u8];
    let mut choice: &[u8];

    while let (Some(c1), Some(c2), Some(newline)) = (matches.next(), matches.next(), matches.next()) {
        user_id_hash = &data[prev_newline..c1];
        choice = &data[c2 + 1..newline];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
        prev_newline = newline;
    }


    // let mut indicator = 0;
    // let mut c1 = 0;
    // let mut c2 = 0;
    // let mut newline = 0;

    // for idx in matches {
    //     match indicator {
    //         0 => {
    //             c1 = idx;
    //             indicator = 1;
    //         },
    //         1 => {
    //             c2 = idx;
    //             indicator = 2;
    //         },
    //         2 => {
    //             newline = idx;
    //             let user_id_hash = &data[prev_newline..c1];
    //             let choice = &data[c2 + 1..newline];

    //             // Overwrite the latest vote for the user
    //             latest_votes.insert(user_id_hash, choice);
    //             prev_newline = newline;
    //             indicator = 0;
    //         },
    //         _ => {}
    //     }
    // }



    // while let Some(comma1) = separators.next() {
        // if let Some(comma2) = separators.next() {
        //     if let Some(newline) = separators.next() {
        //         let user_id_hash = &data[prev_newline..comma1];
        //         let choice = &data[comma2 + 1..newline];

        //         // Overwrite the latest vote for the user
        //         latest_votes.insert(user_id_hash, choice);
        //         prev_newline = newline;
        //     }
        // }

        // comma2 = separators.next();
        // newline = separators.next();
        // if let(Some(comma1), Some(comma2), Some(newline)) = (comma1, comma2, newline) {
        //     let user_id_hash = &data[prev_newline..comma1];
        //     let choice = &data[comma2 + 1..newline];

        //     // Overwrite the latest vote for the user
        //     latest_votes.insert(user_id_hash, choice);
        //     prev_newline = newline;
        // }
        
        // let user_id_hash = &data[prev_newline..comma1];
        // let choice = &data[comma2 + 1..newline];

        // // Overwrite the latest vote for the user
        // latest_votes.insert(user_id_hash, choice);
        // prev_newline = newline;

        // let line = &data[prev_newline..c1];
        // let mut commas = memchr_iter(b',', line);
        // if let Some(c1) = commas.next() {
        //     if let Some(c2) = commas.next() {
        //         let user_id_hash = &line[..c1];
        //         let choice = &line[c2 + 1..];

        //         // Overwrite the latest vote for the user
        //         latest_votes.insert(user_id_hash, choice);
        //     }
        // }
    // }

    // for line in fast_split(&data, b'\n') {
    //     let mut commas = memchr_iter(b',', line);
    //     if let Some(c1) = commas.next() {
    //         if let Some(c2) = commas.next() {
    //             let user_id_hash = &line[..c1];
    //             let choice = &line[c2 + 1..];

    //             // Overwrite the latest vote for the user
    //             latest_votes.insert(user_id_hash, choice);
    //         }
    //     }
    // }


    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_insert(0) += 1;
    }
        

    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_17 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub fn count_votes_18(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let file_size = file.metadata()?.len() as usize;
    let min_bytes_per_line = 32;
    let max_n_lines = file_size / min_bytes_per_line;
    let mut data = Vec::with_capacity(file_size);
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );

    
    for line in fast_split(&data, b'\n') {
        let mut commas = memchr_iter(b',', line);
        if let Some(c1) = commas.next() {
            if let Some(c2) = commas.next() {
                let user_id_hash = &line[..c1];
                let choice = &line[c2 + 1..];

                // // Overwrite the latest vote for the user
                latest_votes.insert(user_id_hash, choice);
            }
        }
    }

    let duration_process = start_process.elapsed();

    // Count the votes
    let start_count = Instant::now();

    let mut counts: FxHashMap<&[u8], u32> = FxHashMap::from_iter(
        choices.iter()
        .map(|choice| (choice.key.as_bytes(), 0))
    );

    for choice in latest_votes.values() {
        *counts.entry(*choice).or_default() += 1;
    }
        
    let duration_count = start_count.elapsed();

    // Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = counts
        .into_iter()
        .map(|(choice, count)| VoteCount {
            choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
            count,
        })
        .collect();

    let duration_total = start_total.elapsed();
    info!("count_votes_18 - total {:?}  -  open and buffer file: {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_read, duration_process, duration_count
    );

    Ok(vote_counts)
}
