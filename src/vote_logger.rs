use std::io::Write;
use tokio::sync::mpsc::Receiver;

pub struct ChannelMessage {
    pub data: Vec<u8>,
    pub resp: tokio::sync::oneshot::Sender<bool>,
}

pub async fn write_lines_to_file(file_path: &str, mut rx: Receiver<ChannelMessage>) -> Result<(), std::io::Error> {
    // Open the file in append mode
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    loop {
        let msg = rx.recv().await.expect("Should receive task not error");
        let line = msg.data;
        file.write_all(&line)?;

        msg.resp.send(true).expect("Should send response");
    }
}


pub struct VLCLMessage {
    pub vl_data: Vec<u8>,
    pub cl_data: Vec<u8>,
    pub resp: tokio::sync::oneshot::Sender<bool>,
}

pub async fn write_cl_vl(mut rx: Receiver<VLCLMessage>) -> Result<(), std::io::Error> {
    // Open the file in append mode
    let mut vl_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("vl.csv")?;

    let mut cl_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("cl.csv")?;

    loop {
        let msg = rx.recv().await.expect("Should receive task not error");
        vl_file.write_all(&msg.vl_data)?;
        cl_file.write_all(&msg.cl_data)?;
        msg.resp.send(true).expect("Should send response");
    }
}
