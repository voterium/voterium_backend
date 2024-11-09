use crate::models::{Choice, VoteCount};
use log::info;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;

const RECORD_SIZE: usize = 33;

pub fn make_choices_lookup(choices: &[Choice]) -> FxHashMap<u8, usize> {
    let mut choice_to_index: FxHashMap<u8, usize> =
        FxHashMap::with_capacity_and_hasher(choices.len(), Default::default());

    for (idx, choice) in choices.iter().enumerate() {
        choice_to_index.insert(choice.key.as_bytes()[0], idx);
    }

    info!("made choice_to_index: {:?}", choice_to_index);
    choice_to_index
}

pub fn user_id_hash_u128_from_bytes(bytes: &[u8]) -> u128 {
    u128::from_le_bytes(bytes.try_into().expect("Invalid user_id_hash length"))
}

pub fn make_latest_votes_hashmap(
    data: &[u8],
    choice_to_idx: FxHashMap<u8, usize>,
) -> FxHashMap<u128, usize> {
    let mut latest_votes = init_latest_votes_hashmap(data);

    for line in data.rchunks_exact(RECORD_SIZE) {
        let user_id_hash = user_id_hash_u128_from_bytes(&line[..16]);

        match latest_votes.entry(user_id_hash) {
            Entry::Occupied(_) => {
                continue; // not the users latest vote
            }
            Entry::Vacant(v) => {
                let choice = line[31];
                let choice_idx = choice_to_idx.get(&choice).unwrap();
                v.insert(*choice_idx);
            }
        }
    }

    info!("made latest_votes. size: {}", latest_votes.len());
    latest_votes
}

pub fn counts_from_latest_votes(
    latest_votes: &FxHashMap<u128, usize>,
    choices: &Vec<Choice>,
) -> Vec<u32> {
    let mut counts = vec![0u32; choices.len()];

    for &choice_idx in latest_votes.values() {
        counts[choice_idx] += 1;
    }

    info!("made counts: {:?}", counts);
    counts
}

pub fn init_seen_hashset(data: &[u8]) -> FxHashSet<u128> {
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    let seen: FxHashSet<u128> =
        FxHashSet::with_capacity_and_hasher(max_n_lines, Default::default());
    seen
}

pub fn init_latest_votes_hashmap(data: &[u8]) -> FxHashMap<u128, usize> {
    let max_n_lines = data.len() / RECORD_SIZE + 1;
    let latest_votes: FxHashMap<u128, usize> =
        FxHashMap::with_capacity_and_hasher(max_n_lines, Default::default());
    latest_votes
}

pub fn indexed_counts_to_vote_counts(counts: &[u32], choices: &[Choice]) -> Vec<VoteCount> {
    choices
        .iter()
        .zip(counts)
        .map(|(choice, &count)| VoteCount {
            choice: choice.key.clone(),
            count,
        })
        .collect()
}
