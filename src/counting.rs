use crate::models::{Choice, VoteCount};
use log::info;
use memchr::memchr_iter;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io::Read;
use std::time::Instant;


fn fast_split(data: &[u8], delimiter: u8) -> impl Iterator<Item = &[u8]> {
    memchr_iter(delimiter, data)
        .scan(0, |start, end| {
            let slice = &data[*start..end];
            *start = end + 1;
            Some(slice)
        })
}


pub fn count_votes(choices: &[Choice]) -> Result<Vec<VoteCount>, std::io::Error> {
    let start_total = Instant::now();

    // Open the file and read it into a buffer
    let start_read = Instant::now();
    let mut file = File::open("cl.csv")?;
    let file_size = file.metadata()?.len() as usize;

    let mut data = Vec::with_capacity(file_size);
    file.read_to_end(&mut data)?;
    
    let duration_read = start_read.elapsed();
    
    let min_bytes_per_line = 32;
    let max_n_lines = file_size / min_bytes_per_line;
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
