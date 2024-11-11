use log::info;
use rustc_hash::FxHashMap;
use std::io::Write;
use tokio::sync::mpsc::Receiver;

use crate::counting::utils::{
    counts_from_latest_votes, indexed_counts_to_vote_counts, make_choices_lookup,
    make_latest_votes_hashmap, user_id_hash_u128_from_bytes,
};
use crate::errors::Result;

pub async fn run_ledger_worker(
    mut rx: Receiver<LedgerWorkerMsg>,
    cl_filepath: impl AsRef<std::path::Path>,
    vl_filepath: impl AsRef<std::path::Path>,
) -> Result<()> {
    let mut cl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cl_filepath)?;

    let mut vl = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(vl_filepath)?;

    info!("Ledger Worker started");
    loop {
        let msg = rx.recv().await.expect("Should receive task not error");

        cl.write_all(&msg.cl_line)?;
        vl.write_all(&msg.vl_line)?;

        if let Some(rx) = msg.resp {
            // If the caller provided a response channel, send a response
            rx.send(true).expect("Should send response");
        }
    }
}

use crate::counting::{count_votes, load_cl};
use crate::models::{Choice, CountWorkerBallot, CountWorkerMsg, LedgerWorkerMsg, Vote, VoteCount};

pub async fn run_counts_worker(
    mut rx: Receiver<CountWorkerMsg>,
    cl_filepath: impl AsRef<std::path::Path>,
    choices: &Vec<Choice>,
) -> Result<()> {
    // The Counts Worker maintains a live vote count in memory and updates them
    // as new votes come in so that Voterium can quickly respond to requests
    // for the current count

    let choice_idx_map = make_choices_lookup(choices);

    let mut latest_votes = {
        let cl_data = load_cl(cl_filepath)?;
        make_latest_votes_hashmap(&cl_data, choice_idx_map.clone())
    };

    let mut vote_counts = {
        let counts = counts_from_latest_votes(&latest_votes, choices);
        indexed_counts_to_vote_counts(&counts, choices)
    };

    info!("Counts Worker started. Initial counts: {:?}", vote_counts);
    loop {
        let msg: CountWorkerMsg = rx.recv().await.expect("Should receive task not error");
        match msg {

            CountWorkerMsg::Vote { ballot } => {
                add_vote(
                    &ballot,
                    &choice_idx_map,
                    &mut vote_counts,
                    &mut latest_votes,
                );
            }

            CountWorkerMsg::GetCounts { resp } => {
                resp.send(vote_counts.clone()).expect("Should send response");
            }
        }
    }
}

fn add_vote(
    ballot: &CountWorkerBallot,
    choice_idx_map: &FxHashMap<u8, usize>,
    vote_counts: &mut Vec<VoteCount>,
    latest_votes: &mut FxHashMap<u128, usize>,
) {
    if let Some(&choice_idx) = choice_idx_map.get(&ballot.choice_key) {
        // choice_idx_map turns the choice key into an index so that
        // we can count votes in a Vec (fast) instead of a HashMap (slow)
        let old_choice_idx = latest_votes.insert(ballot.user_id_hash, choice_idx);

        if let Some(old_choice_idx) = old_choice_idx {
            vote_counts[old_choice_idx].count -= 1;
        }

        vote_counts[choice_idx].count += 1;
    }
}
