use std::io::{BufWriter, Write};
use tokio::sync::mpsc::Receiver;

use crate::Result;

pub struct VLCLMessage {
    pub vl_data: Vec<u8>,
    pub cl_data: Vec<u8>,
    pub resp: tokio::sync::oneshot::Sender<bool>,
}

pub async fn write_cl_vl(mut rx: Receiver<VLCLMessage>) -> Result<()> {
    // Open the file in append mode
    let mut vl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("vl.csv")?;

    // let mut vl = BufWriter::new(vl_file);

    let mut cl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("cl.csv")?;

    // let mut cl = BufWriter::new(cl_file);

    loop {
        let msg = rx.recv().await.expect("Should receive task not error");
        // vl.write(&msg.vl_data)?;
        // cl.write(&msg.cl_data)?;
        vl.write_all(&msg.vl_data)?;
        cl.write_all(&msg.cl_data)?;
        // msg.resp.send(true).expect("Should send response");
    }
}
