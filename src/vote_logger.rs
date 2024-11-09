use std::io::Write;
use log::info;
use rustc_hash::FxHashMap;
use tokio::sync::mpsc::Receiver;

use crate::counting::utils::{counts_from_latest_votes, indexed_counts_to_vote_counts, make_choices_lookup, make_latest_votes_hashmap, user_id_hash_u128_from_bytes};
use crate::errors::Result;

type CLLine = Vec<u8>;

pub struct VLCLMessage {
    pub vl_data: Vec<u8>,
    pub cl_data: CLLine,
    // pub resp: tokio::sync::oneshot::Sender<bool>,
}

pub async fn write_cl_vl(mut rx: Receiver<VLCLMessage>) -> Result<()> {
    let mut vl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("vl.csv")?;

    let mut cl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("cl.csv")?;

    loop {
        let msg = rx.recv().await.expect("Should receive task not error");
        vl.write_all(&msg.vl_data)?;
        cl.write_all(&msg.cl_data)?;
        // msg.resp.send(true).expect("Should send response");
    }
}

use crate::counting::{count_votes, load_cl};
use crate::models::{Choice, Vote, VoteCount};


pub enum CountsCacheMsg {
    Vote{cl_line: CLLine},
    GetCounts{resp: tokio::sync::oneshot::Sender<Vec<VoteCount>>},
}

pub async fn vote_counts_cache(mut rx: Receiver<CountsCacheMsg>, cl_filepath: impl AsRef<std::path::Path>, choices: &Vec<Choice>) -> Result<()> {
    let choices_lookup = make_choices_lookup(choices);

    let mut latest_votes = {
        let cl_data = load_cl(cl_filepath)?;
        make_latest_votes_hashmap(&cl_data, choices_lookup.clone())
    };

    let mut vote_counts = {
        let counts = counts_from_latest_votes(&latest_votes, choices);
        indexed_counts_to_vote_counts(&counts, choices)
    };

    loop {
        let msg: CountsCacheMsg = rx.recv().await.expect("Should receive task not error");
        match msg {
            CountsCacheMsg::Vote{cl_line} => {
                add_vote(&cl_line, &choices_lookup, &mut vote_counts, &mut latest_votes);
            },
            CountsCacheMsg::GetCounts{resp} => {
                resp.send(vote_counts.clone()).expect("Should send response");
            }
        }
    }
}

fn add_vote(cl_line: &CLLine, choice_index_map: &FxHashMap<u8, usize>, vote_counts: &mut Vec<VoteCount>, latest_votes: &mut FxHashMap<u128, usize>) {
    let user_id_hash = user_id_hash_u128_from_bytes(&cl_line[..16]);
    let choice = cl_line[31];
    if let Some(&choice_idx) = choice_index_map.get(&choice) {
        let old_choice_idx = latest_votes.insert(user_id_hash, choice_idx);
        if let Some(old_choice_idx) = old_choice_idx {
            vote_counts[old_choice_idx].count -= 1;
        }
        vote_counts[choice_idx].count += 1;
    }
}
