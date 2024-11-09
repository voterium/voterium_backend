// Various implementations of vote counting to compare performance

use crate::models::{Choice, VoteCount};
use log::info;
use memchr::{memchr, memchr2_iter, memchr_iter};
use rustc_hash::{FxHashMap, FxHashSet};
use core::str;
use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::io::{BufReader, Read};
use std::time::Instant;

use crate::errors::Result;



#[derive(Clone)]
pub struct CLVote {
    pub user_id_hash: String,
    pub timestamp: i64,
    pub choice: String,
}

#[allow(dead_code)]
fn data_to_lines(data: &[u8]) -> impl Iterator<Item = &str> {
    // Attempt to convert the entire byte slice to a &str
    let text = str::from_utf8(data).expect("Data is not valid UTF-8");
    
    // Split the &str on '\n' and collect into a vector
    text.split('\n')
}

#[allow(dead_code)]
pub fn count_votes_01(data: &[u8], ) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
    let mut votes: Vec<CLVote> = Vec::new();

    // Parse lines
    let start_parse = Instant::now();
    for line in data_to_lines(data) {

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
    info!("count_votes_1 - total {:?}  -  parse lines and collect votes {:?}, process votes into latest_votes {:?}, count votes {:?}",
            duration_total, duration_parse, duration_process, duration_count);

    Ok(vote_counts)
}

// pub fn count_votes_02(data: &[u8], ) -> Result<Vec<VoteCount>> {
//     use std::time::Instant;

//     let start_total = Instant::now();

//     // Read the entire file into a String to own the data
//     let start_read = Instant::now();
//     let mut file = File::open("cl.csv")?;
//     let mut contents = String::new();
//     file.read_to_string(&mut contents)?;
//     let duration_read = start_read.elapsed();

//     // Create a CSV reader from the string
//     let start_parse = Instant::now();
//     let mut rdr = ReaderBuilder::new()
//         .has_headers(false)
//         .trim(csv::Trim::All)
//         .from_reader(contents.as_bytes());
//     let duration_parse_setup = start_parse.elapsed();

//     // Process records
//     let start_process = Instant::now();
//     let mut latest_votes: HashMap<String, String> = HashMap::new();

//     for result in rdr.records() {
//         let record = result?;

//         if record.len() != 3 {
//             continue; // Skip malformed lines
//         }

//         let user_id_hash = record.get(0).unwrap().to_string();
//         // We can ignore timestamp since votes are sorted
//         let choice = record.get(2).unwrap().to_string();

//         // Overwrite the latest vote for the user
//         latest_votes.insert(user_id_hash, choice);
//     }
//     let duration_process = start_process.elapsed();

//     // Count the votes
//     let start_count = Instant::now();
//     let mut counts: HashMap<String, u32> = HashMap::new();
//     for choice in latest_votes.values() {
//         *counts.entry(choice.clone()).or_insert(0) += 1;
//     }
//     let duration_count = start_count.elapsed();

//     // Convert counts to a vector of VoteCount
//     let vote_counts: Vec<VoteCount> = counts
//         .into_iter()
//         .map(|(choice, count)| VoteCount { choice, count })
//         .collect();

//     let duration_total = start_total.elapsed();
//     info!("count_votes_2 - total {:?}, parse lines and collect votes {:?}, process votes into latest_votes {:?}, count votes {:?}",
//              duration_total, duration_parse_setup, duration_process, duration_count);

//     Ok(vote_counts)
// }

#[allow(dead_code)]
pub fn count_votes_03(data: &[u8], ) -> Result<Vec<VoteCount>> {
    use std::time::Instant;

    let start_total = Instant::now();

    // Since we know the file size, we can preallocate buffers
    let start_process = Instant::now();

    // Prepare buffers for lines and parts to reduce allocations
    let mut latest_votes: HashMap<String, String> = HashMap::new();

    let lines = data_to_lines(data);

    for line in lines {
        let line_trimmed = line.trim();

        // Find indices of the commas
        let mut parts_iter = line_trimmed.splitn(3, ',');
        let user_id_hash = match parts_iter.next() {
            Some(s) => s,
            None => {
                continue; // Skip malformed lines
            }
        };

        // Skip the timestamp since data is sorted
        parts_iter.next();

        let choice = match parts_iter.next() {
            Some(s) => s,
            None => {
                continue; // Skip malformed lines
            }
        };

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash.to_string(), choice.to_string());

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
    info!("count_votes_3 - total {:?} process votes to latest_votes {:?}, count votes {:?}",
             duration_total, duration_process, duration_count);

    Ok(vote_counts)
}

#[allow(dead_code)]
fn count_lines_fast(reader: &mut BufReader<File>) -> Result<usize> {
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

pub fn count_votes_04(data: &[u8], ) -> Result<Vec<VoteCount>> {
    use std::time::Instant;

    let start_total = Instant::now();
    let start_process = Instant::now();

    // Prepare buffers for lines and parts to reduce allocations
    let line_count = data.len() / 33; // Average line length is 33 bytes
    let mut latest_votes: FxHashMap<String, String> = FxHashMap::default();
    latest_votes.reserve(line_count);

    let lines = data_to_lines(data);

    for line in lines {
        let line_trimmed = line.trim();

        // Find indices of the commas
        let mut parts_iter = line_trimmed.splitn(3, ',');
        let user_id_hash = match parts_iter.next() {
            Some(s) => s,
            None => {
                continue; // Skip malformed lines
            }
        };

        // Skip the timestamp since data is sorted
        parts_iter.next();

        let choice = match parts_iter.next() {
            Some(s) => s,
            None => {
                continue; // Skip malformed lines
            }
        };

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash.to_string(), choice.to_string());

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
    info!("count_votes_4 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}

// pub fn count_votes_05(data: &[u8], ) -> Result<Vec<VoteCount>> {
//     let start_total = Instant::now();

//     // Open the file and memory-map it
//     let start_read = Instant::now();
//     let file = File::open("cl.csv")?;
//     let mmap = unsafe { Mmap::map(&file)? };
//     let data = &*mmap;
//     let duration_read = start_read.elapsed();

//     let start_process = Instant::now();

//     let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();

//     let mut i = 0;
//     let data_len = data.len();

//     while i < data_len {
//         // Find the next newline
//         let next_newline = memchr(b'\n', &data[i..]).unwrap_or(data_len - i);
//         let line = &data[i..i + next_newline];
//         i += next_newline + 1; // Move past the newline

//         // Split the line by commas
//         let mut start = 0;
//         let mut parts = Vec::with_capacity(3);
//         for _ in 0..3 {
//             match memchr(b',', &line[start..]) {
//                 Some(pos) => {
//                     parts.push(&line[start..start + pos]);
//                     start += pos + 1;
//                 }
//                 None => {
//                     parts.push(&line[start..]);
//                     break;
//                 }
//             }
//         }
//         if parts.len() < 3 {
//             // Skip malformed line
//             continue;
//         }

//         let user_id_hash = parts[0];
//         // Skip timestamp (parts[1])
//         let choice = parts[2];

//         // Overwrite the latest vote for the user
//         latest_votes.insert(user_id_hash, choice);
//     }
//     let duration_process = start_process.elapsed();

//     // Count the votes
//     let start_count = Instant::now();
//     let mut counts: HashMap<&[u8], u32> = HashMap::new();
//     for choice in latest_votes.values() {
//         *counts.entry(*choice).or_insert(0) += 1;
//     }
//     let duration_count = start_count.elapsed();

//     // Convert counts to a vector of VoteCount
//     let vote_counts: Vec<VoteCount> = counts
//         .into_iter()
//         .map(|(choice, count)| VoteCount {
//             choice: std::str::from_utf8(choice).unwrap_or("").to_string(),
//             count,
//         })
//         .collect();

//     let duration_total = start_total.elapsed();
//     info!("count_votes_5 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
//         duration_total, duration_process, duration_count
//     );

//     Ok(vote_counts)
// }

#[allow(dead_code)]
pub fn count_votes_06(data: &[u8], ) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_6 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


// pub fn count_votes_07(data: &[u8], ) -> Result<Vec<VoteCount>> {
//     let start_total = Instant::now();

//     // Open the file
//     let start_read = Instant::now();
//     let mut file = File::open("cl.csv")?;
//     let duration_read = start_read.elapsed();

//     let start_process = Instant::now();

//     let mut latest_votes: FxHashMap<Vec<u8>, Vec<u8>> = FxHashMap::default();

//     const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer size
//     let mut buf = vec![0u8; BUFFER_SIZE];
//     let mut bytes_in_buffer = 0;

//     loop {
//         // Read data into buf[bytes_in_buffer..]
//         let bytes_read = file.read(&mut buf[bytes_in_buffer..])?;
//         if bytes_read == 0 {
//             break; // EOF reached
//         }
//         bytes_in_buffer += bytes_read;
//         let buf_slice = &buf[..bytes_in_buffer];

//         // Find the last newline in buf_slice
//         let last_newline_index = match memrchr(b'\n', buf_slice) {
//             Some(index) => index,
//             None => {
//                 // No newline found in buffer
//                 if bytes_in_buffer == buf.len() {
//                     // Buffer full but no newline, likely a very long line
//                     return Err(AppError::InternalError{ title: "Line too long".to_string(), message: "A line in the file is too long to process".to_string() });
//                 }
//                 continue; // Read more data to find a newline
//             }
//         };

//         // Process complete lines in buf_slice[..=last_newline_index]
//         let lines = &buf_slice[..=last_newline_index]; // Include the newline

//         let mut start = 0;
//         while start < lines.len() {
//             // Find the next newline
//             let next_newline = memchr(b'\n', &lines[start..]).unwrap();
//             let line = &lines[start..start + next_newline]; // Exclude the newline
//             start += next_newline + 1; // Move past the newline

//             // Split the line by commas
//             let mut field_start = 0;
//             let mut parts = Vec::with_capacity(3);
//             for _ in 0..3 {
//                 match memchr(b',', &line[field_start..]) {
//                     Some(pos) => {
//                         parts.push(&line[field_start..field_start + pos]);
//                         field_start += pos + 1;
//                     }
//                     None => {
//                         parts.push(&line[field_start..]);
//                         break;
//                     }
//                 }
//             }
//             if parts.len() < 3 {
//                 // Skip malformed line
//                 continue;
//             }

//             let user_id_hash = parts[0].to_vec();
//             // Skip timestamp (parts[1])
//             let choice = parts[2].to_vec();

//             // Overwrite the latest vote for the user
//             latest_votes.insert(user_id_hash, choice);
//         }

//         // Copy any remaining bytes after last_newline_index to the beginning of buf
//         let remaining = bytes_in_buffer - (last_newline_index + 1);
//         buf.copy_within((last_newline_index + 1)..bytes_in_buffer, 0);
//         bytes_in_buffer = remaining;
//     }

//     // After the loop, process any remaining data in buf[..bytes_in_buffer]
//     if bytes_in_buffer > 0 {
//         let line = &buf[..bytes_in_buffer];

//         // Split the line by commas
//         let mut field_start = 0;
//         let mut parts = Vec::with_capacity(3);
//         for _ in 0..3 {
//             match memchr(b',', &line[field_start..]) {
//                 Some(pos) => {
//                     parts.push(&line[field_start..field_start + pos]);
//                     field_start += pos + 1;
//                 }
//                 None => {
//                     parts.push(&line[field_start..]);
//                     break;
//                 }
//             }
//         }
//         if parts.len() >= 3 {
//             let user_id_hash = parts[0].to_vec();
//             // Skip timestamp (parts[1])
//             let choice = parts[2].to_vec();

//             // Overwrite the latest vote for the user
//             latest_votes.insert(user_id_hash, choice);
//         }
//     }

//     let duration_process = start_process.elapsed();

//     // Count the votes
//     let start_count = Instant::now();
//     let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
//     for choice in latest_votes.values() {
//         *counts.entry(choice.clone()).or_insert(0) += 1;
//     }
//     let duration_count = start_count.elapsed();

//     // Convert counts to a vector of VoteCount
//     let vote_counts: Vec<VoteCount> = counts
//         .into_iter()
//         .map(|(choice, count)| VoteCount {
//             choice: String::from_utf8(choice).unwrap_or_default(),
//             count,
//         })
//         .collect();

//     let duration_total = start_total.elapsed();
//     info!("count_votes_7 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
//         duration_total, duration_process, duration_count
//     );

//     Ok(vote_counts)
// }


#[allow(dead_code)]
pub fn count_votes_08(data: &[u8], ) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
    latest_votes.reserve(data.len() / 33); // Average line length is 33 bytes

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
    info!("count_votes_8 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}

fn find_new_line_pos(bytes: &[u8]) -> Option<usize> {
    // In this case (position is not far enough),
    // naive version is faster than bstr (memchr)
    bytes.iter().rposition(|&b| b == b'\n')
}

// pub fn count_votes_09(data: &[u8], ) -> Result<Vec<VoteCount>> {
//     let start_total = Instant::now();

//     // Open the file and read it into a buffer
//     let start_read = Instant::now();
//     let mut file = File::open("cl.csv")?;
//     let duration_read = start_read.elapsed();

//     let start_process = Instant::now();

//     // let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
//     let mut latest_votes: FxHashMap<Vec<u8>, Vec<u8>> = FxHashMap::default();

//     const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer size

//     let mut buf = vec![0u8; BUFFER_SIZE];
//     let mut bytes_not_processed = 0;

//     while let Ok(n_bytes_read) = file.read(&mut buf[bytes_not_processed..]) {
//         if n_bytes_read == 0 {
//             break; // EOF reached
//         }

//         let actual_buf = &mut buf[..bytes_not_processed + n_bytes_read];
//         let last_new_line_index = match find_new_line_pos(&actual_buf) {
//             Some(index) => index,
//             None => {
//                 warn!("No new line found in the read buffer");
//                 bytes_not_processed += n_bytes_read;
//                 if bytes_not_processed == buf.len() {
//                     panic!("No new line found in the read buffer");
//                 }
//                 continue; // try again, maybe we next read will have a newline
//             }
//         };

//         let chunk = &actual_buf[..last_new_line_index + 1];

//         for line in chunk.split(|&b| b == b'\n') {
//             let mut start = 0;
//             let mut parts = Vec::with_capacity(3);
//             for _ in 0..3 {
//                 match memchr(b',', &line[start..]) {
//                     Some(pos) => {
//                         parts.push(&line[start..start + pos]);
//                         start += pos + 1;
//                     }
//                     None => {
//                         parts.push(&line[start..]);
//                         break;
//                     }
//                 }
//             }
//             if parts.len() < 3 {
//                 // Skip malformed line
//                 bytes_not_processed = &actual_buf.len() - last_new_line_index - 1;
//                 continue;
//             }

//             let user_id_hash = parts[0].to_vec();
//             // Skip timestamp (parts[1])
//             let choice = parts[2].to_vec();

//             // Overwrite the latest vote for the user
//             latest_votes.insert(user_id_hash, choice);
//         }

//         actual_buf.copy_within(last_new_line_index + 1.., 0);
//         // You cannot use bytes_not_processed = bytes_read - last_new_line_index
//         // - 1; because the buffer will contain unprocessed bytes from the
//         // previous iteration and the new line index will be calculated from the
//         // start of the buffer
//         bytes_not_processed = actual_buf.len() - last_new_line_index - 1;
//     }
//     let duration_process = start_process.elapsed();

//     // Count the votes
//     let start_count = Instant::now();
//     let mut counts: HashMap<Vec<u8>, u32> = HashMap::new();
//     for choice in latest_votes.values() {
//         *counts.entry(choice.clone()).or_insert(0) += 1;
//     }
//     let duration_count = start_count.elapsed();

//     // Convert counts to a vector of VoteCount
//     let vote_counts: Vec<VoteCount> = counts
//         .into_iter()
//         .map(|(choice, count)| VoteCount {
//             choice: String::from_utf8(choice).unwrap_or_default(),
//             count,
//         })
//         .collect();

//     let duration_total = start_total.elapsed();
//     info!("count_votes_9 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
//         duration_total, duration_process, duration_count
//     );

//     Ok(vote_counts)
// }

#[allow(dead_code)]
pub fn count_votes_10(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_10 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_11(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
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
    info!("count_votes_11 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_12(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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

    let choice_keys = choices.iter().map(|choice| choice.key.as_bytes()).collect::<Vec<_>>();
    for choice in latest_votes.iter().filter_map(|(_, value)| 
        if choice_keys.contains(value) {
            Some(value)
        } else {
            None
        }
    ) {
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
    info!("count_votes_12 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}

#[allow(dead_code)]
pub fn count_votes_13(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_13 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_14(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_14 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}

#[allow(dead_code)]
pub fn count_votes_15(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_15 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
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


#[allow(dead_code)]
pub fn count_votes_16(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

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
    info!("count_votes_16 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_17(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::default();
    
    
    let mut matches =  memchr2_iter(b'\n', b',', &data);    
    let mut prev_newline = 0;

    let mut user_id_hash: &[u8];
    let mut choice: &[u8];

    while let (Some(c1), Some(c2), Some(newline)) = (matches.next(), matches.next(), matches.next()) {
        user_id_hash = &data[prev_newline..c1];
        choice = &data[c2 + 1..newline];

        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
        prev_newline = newline;
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
    info!("count_votes_17 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_18(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();


    let max_n_lines = data.len() / 33; // Average line length is 33 bytes
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
    info!("count_votes_18 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_19(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
    
    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );

    fast_split(&data, b'\n')
        .for_each(|line| {
            let mut commas = memchr_iter(b',', line);

            let Some(c1) = commas.next() else { return };
            let Some(c2) = commas.next() else { return };
            let user_id_hash = &line[..c1];
            let choice = &line[c2 + 1..];

            latest_votes.insert(user_id_hash, choice);
        });


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
    info!("count_votes_19 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_20(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
    
    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );

    let mut user_id_hash: &[u8];
    let mut choice: &[u8];
    let mut commas: memchr::Memchr;

    for line in fast_split(&data, b'\n') {
        commas = memchr_iter(b',', line);

        user_id_hash = match commas.next() {
            Some(comma) => &line[..comma],
            None => continue,
        };

        choice = match commas.next() {
            Some(comma) => &line[comma + 1..],
            None => continue,
        };


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
    info!("count_votes_20 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


pub enum MatchChar {
    Comma1,
    Comma2,
    Newline,
}

#[allow(dead_code)]
pub fn count_votes_21(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
    
    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );

    let mut matches =  memchr2_iter(b'\n', b',', &data);    
    
    let mut prev_newline = 0;
    let mut user_id_hash: &[u8];
    let mut choice: &[u8];
    while let (Some(c1), Some(c2), Some(newline)) = (matches.next(), matches.next(), matches.next()) {
        user_id_hash = &data[prev_newline..c1];
        choice = &data[c2 + 1..newline];
    
        // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash, choice);
        prev_newline = newline;
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
    info!("count_votes_21 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_22(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();
    
    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let start_process = Instant::now();

    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );

    fast_split(data, b'\n')
        .filter_map(|line| {
            let mut commas = memchr_iter(b',', line);

            let c1 = commas.next()?;
            let c2 = commas.next()?;
            let user_id_hash = &line[..c1];
            let choice = &line[c2 + 1..];
            Some((user_id_hash, choice))
        }).for_each(|(user_id_hash, choice)| {
            latest_votes.insert(user_id_hash, choice);
        });


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
    info!("count_votes_22 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_23(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let mut latest_votes: FxHashMap<&[u8], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );


    for line in fast_split(&data, b'\n') {
        let user_id_hash = &line[..16];
        let choice = &line[31..];

        // // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash , choice);
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
    info!("count_votes_23 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_24(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let mut latest_votes: FxHashMap<[u8; 16], &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );


    let mut user_id_hash: [u8; 16] = [0; 16];
    for line in fast_split(&data, b'\n') {
        user_id_hash.copy_from_slice(&line[..16]);
        let choice = &line[31..];

        // // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash , choice);
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
    info!("count_votes_24 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_25(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line;
    let mut latest_votes: FxHashMap<u128, &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );


    // let mut user_id_hash: [u8; 16] = [0; 16];
    for line in fast_split(&data, b'\n') {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Line too short for user_id_hash")
        );
        let choice = &line[31..];

        // // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash , choice);
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
    info!("count_votes_25 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


use bstr::ByteSlice;

#[allow(dead_code)]
pub fn count_votes_26(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    let start_process = Instant::now();

    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line; // Average line length is 33 bytes
    let mut latest_votes: FxHashMap<u128, &[u8]> = FxHashMap::with_capacity_and_hasher(
        max_n_lines, 
        Default::default()
    );


    // let mut user_id_hash: [u8; 16] = [0; 16];
    for line in data[..].split_str("\n") {
        if line.len() < 32 {
            continue;
        }

        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Line too short for user_id_hash")
        );
        let choice = &line[31..];

        // // Overwrite the latest vote for the user
        latest_votes.insert(user_id_hash , choice);
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
    info!("count_votes_26 - total {:?}, process votes to latest_votes {:?}, count votes {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_27(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<&[u8], usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes(), idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    let min_bytes_per_line = 32;
    let max_n_lines = data.len() / min_bytes_per_line; // Average line length is 33 bytes
    // Using u128 as key for better performance
    let mut latest_votes: FxHashMap<u128, usize> = FxHashMap::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    for line in fast_split(&data, b'\n') {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );

        let choice_bytes = &line[31..]; // Adjust indices as per your data format

        // Lookup the choice index
        if let Some(&choice_idx) = choice_to_index.get(choice_bytes) {
            // Overwrite the latest vote for the user with the choice index
            latest_votes.insert(user_id_hash, choice_idx);
        }
        // If the choice_bytes do not correspond to any known choice, you can handle it accordingly
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_27 - total {:?}, choice mapping: {:?}, process: {:?}, count: {:?}",
        duration_total, duration_choice_mapping, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_28(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<&[u8], usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes(), idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1; 
    // Using u128 as key for better performance
    let mut latest_votes: FxHashMap<u128, usize> = FxHashMap::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    for line in data.chunks_exact(RECORD_SIZE) {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );

        let choice_bytes = &line[31..32]; // Adjust indices as per your data format

        // Lookup the choice index
        if let Some(&choice_idx) = choice_to_index.get(choice_bytes) {
            // Overwrite the latest vote for the user with the choice index
            latest_votes.insert(user_id_hash, choice_idx);
        }
        // If the choice_bytes do not correspond to any known choice, you can handle it accordingly
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_28 - total {:?}, choice mapping: {:?}, process: {:?}, count: {:?}",
        duration_total, duration_choice_mapping, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_29(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<&[u8], usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes(), idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    let mut latest_votes: FxHashMap<u128, usize> = FxHashMap::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    for line in data.chunks_exact(RECORD_SIZE) {
        let user_id_hash = u128::from_le_bytes([
            line[0], line[1], line[2], line[3],
            line[4], line[5], line[6], line[7],
            line[8], line[9], line[10], line[11],
            line[12], line[13], line[14], line[15],
        ]);

        let choice_bytes = &line[31..32]; // Adjust indices as per your data format

        // Lookup the choice index
        if let Some(&choice_idx) = choice_to_index.get(choice_bytes) {
            // Overwrite the latest vote for the user with the choice index
            latest_votes.insert(user_id_hash, choice_idx);
        }
        // If the choice_bytes do not correspond to any known choice, you can handle it accordingly
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_29 - total {:?}, choice mapping: {:?}, process: {:?}, count: {:?}",
        duration_total, duration_choice_mapping, duration_process, duration_count
    );

    Ok(vote_counts)
}


use ahash::AHashMap;

#[allow(dead_code)]
pub fn count_votes_30(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<&[u8], usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes(), idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    let mut latest_votes: AHashMap<u128, usize> = AHashMap::with_capacity(max_n_lines);

    for line in data.chunks_exact(RECORD_SIZE) {
        let user_id_hash = u128::from_le_bytes([
            line[0], line[1], line[2], line[3],
            line[4], line[5], line[6], line[7],
            line[8], line[9], line[10], line[11],
            line[12], line[13], line[14], line[15],
        ]);

        let choice_bytes = &line[31..32]; // Adjust indices as per your data format

        // Lookup the choice index
        if let Some(&choice_idx) = choice_to_index.get(choice_bytes) {
            // Overwrite the latest vote for the user with the choice index
            latest_votes.insert(user_id_hash, choice_idx);
        }
        // If the choice_bytes do not correspond to any known choice, you can handle it accordingly
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_30 - total {:?}, choice mapping: {:?}, process: {:?}, count: {:?}",
        duration_total, duration_choice_mapping, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_31(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<u8, usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes()[0], idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();


    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    // Using u128 as key for better performance
    let mut latest_votes: FxHashMap<u128, usize> = FxHashMap::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    for line in data.chunks_exact(RECORD_SIZE) {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );

        let choice_byte = &line[31]; // Adjust indices as per your data format

        // Lookup the choice index
        if let Some(&choice_idx) = choice_to_index.get(choice_byte) {
            // Overwrite the latest vote for the user with the choice index
            latest_votes.insert(user_id_hash, choice_idx);
        }
        // If the choice_bytes do not correspond to any known choice, you can handle it accordingly
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_31 - total  {:?} choice mapping: {:?}, process: {:?}, count: {:?}",
        duration_total, duration_choice_mapping, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_32(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    // depends on the `choices` keys being sequential array index
    // integers (0, 1, 2, ...)

    let start_total = Instant::now();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    // Using u128 as key for better performance
    let mut latest_votes: FxHashMap<u128, usize> = FxHashMap::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    for line in data.chunks_exact(RECORD_SIZE) {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );

        let choice_idx = (line[31] - b'0') as usize;

        latest_votes.insert(user_id_hash, choice_idx);
    }

    let duration_process = start_process.elapsed();

    // Step 4: Count the votes
    let start_count = Instant::now();

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    // Iterate over latest_votes and increment counts
    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    let duration_count = start_count.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_32 - total {:?}, process: {:?}, count: {:?}",
        duration_total, duration_process, duration_count
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_33(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {
    // depends on the `choices` keys being sequential array index
    // integers (0, 1, 2, ...)

    let start_total = Instant::now();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    
    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    // // Using u128 as key for better performance
    let mut seen: FxHashSet<u128> = FxHashSet::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    for line in data.chunks_exact(RECORD_SIZE).rev() {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );


        if let Some(_) = seen.replace(user_id_hash) {
            // this is not the voters latest vote
            continue
        }

        let choice_idx = (line[31] - b'0') as usize;
        counts[choice_idx] += 1;

    }

    let duration_process = start_process.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_33 - total {:?}, process: {:?}",
        duration_total, duration_process
    );

    Ok(vote_counts)
}


#[allow(dead_code)]
pub fn count_votes_34(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {

    let start_total = Instant::now();

    // Step 2: Create a mapping from choice key to index
    let start_choice_mapping = Instant::now();
    let mut choice_to_index: FxHashMap<u8, usize> = FxHashMap::with_capacity_and_hasher(
        choices.len(),
        Default::default(),
    );

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes()[0], idx);
    }
    let duration_choice_mapping = start_choice_mapping.elapsed();

    // Step 3: Process lines to determine the latest vote per user
    let start_process = Instant::now();

    // // Using u128 as key for better performance
    const RECORD_SIZE: usize = 33;
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    let mut seen: FxHashSet<u128> = FxHashSet::with_capacity_and_hasher(
        max_n_lines,
        Default::default(),
    );

    // Initialize a counts vector
    let mut counts = vec![0u32; choices.len()];

    for line in data.chunks_exact(RECORD_SIZE).rev() {
        let user_id_hash = u128::from_le_bytes(
            line[..16].try_into().expect("Invalid user_id_hash length"),
        );


        if let Some(_) = seen.replace(user_id_hash) {
            // this is not the voters latest vote
            continue
        }

        if let Some(choice_idx) = choice_to_index.get(&line[31]) {
            counts[*choice_idx] += 1;
        }
    }

    let duration_process = start_process.elapsed();

    // Step 5: Convert counts to a vector of VoteCount
    let vote_counts: Vec<VoteCount> = choices.iter()
        .enumerate()
        .map(|(idx, choice)| VoteCount {
            choice: choice.key.clone(),
            count: counts[idx],
        })
        .collect();

    let duration_total = start_total.elapsed();

    info!(
        "count_votes_34 - total {:?}, map choices {:?}, process: {:?}",
        duration_total, duration_choice_mapping, duration_process
    );

    Ok(vote_counts)
}



use crate::counting::utils::{make_choices_lookup, init_seen_hashset, user_id_hash_u128_from_bytes};
use super::utils::indexed_counts_to_vote_counts;
const RECORD_SIZE: usize = 33;

#[allow(dead_code)]
pub fn count_votes_35(data: &[u8], choices: &[Choice]) -> Result<Vec<VoteCount>> {

    let choice_to_index = make_choices_lookup(choices);
    let mut seen_users = init_seen_hashset(data);
    let mut counts = vec![0u32; choices.len()];

    for line in data.chunks_exact(RECORD_SIZE).rev() {
        let user_id_hash = user_id_hash_u128_from_bytes(&line[..16]);

        let is_latest_vote = seen_users.insert(user_id_hash);
        if !is_latest_vote {
            continue
        }

        let choice  = line[31];
        if let Some(choice_idx) = choice_to_index.get(&choice) {
            counts[*choice_idx] += 1;
        }
    }

    let vote_counts = indexed_counts_to_vote_counts(&counts, choices);

    Ok(vote_counts)
}

