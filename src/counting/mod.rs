pub mod counting_funcs;
mod tests;
use crate::errors::Result;
use std::{fs::File, io::Read, path::Path, time::Instant};

pub use counting_funcs::count_votes_34 as count_votes;
use log::info;

pub fn load_data(filepath: impl AsRef<Path>) -> Result<Vec<u8>> {
    let start_read = Instant::now();
    let mut file = File::open(filepath)?;
    let file_size = file.metadata()?.len() as usize;
    let mut data = Vec::with_capacity(file_size);
    file.read_to_end(&mut data)?;
    let duration_read = start_read.elapsed();
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
    
    info!("load_data - read {:.2} MB in {:?}", file_size_mb, duration_read);
    
    Ok(data)
}
