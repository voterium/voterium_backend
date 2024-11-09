
// Assuming this test module is within the same crate where count_votes_* functions are defined.
// Adjust the `use` paths if the functions are located in different modules.

#[cfg(test)]
mod tests {
    use crate::counting::counting_funcs::*; // Import all items from the parent module.
    use crate::models::{Choice, VoteCount};
    use crate::counting::load_cl;
    use crate::errors::Result;

    /// Loads the voting configuration and extracts the choices.
    fn make_input() -> Vec<Choice> {
        vec![
            Choice { key: "0".to_string(), label: "Choice 0".to_string(), color: "#FF0000".to_string() },
            Choice { key: "1".to_string(), label: "Choice 1".to_string(), color: "#FF0000".to_string() },
            Choice { key: "2".to_string(), label: "Choice 2".to_string(), color: "#FF0000".to_string() },
        ]
    }

    #[test]
    fn test_all_count_votes_functions_return_same_value() -> Result<()> {
        // Load choices
        let choices = make_input();
        let data = load_cl("cl_1M_int.csv").unwrap();

        fn sorted(vote_counts: Vec<VoteCount>) -> Vec<VoteCount> {
            let mut sorted_counts = vote_counts.clone();
            sorted_counts.sort_by(|a, b| a.choice.cmp(&b.choice));
            sorted_counts
        }

        let reference_counts = sorted(count_votes_34(&data, &choices)?);

        assert_eq!(sorted(count_votes_01(&data)?), reference_counts);
        assert_eq!(sorted(count_votes_03(&data)?), reference_counts);
        assert_eq!(sorted(count_votes_04(&data)?), reference_counts);
        assert_eq!(sorted(count_votes_06(&data)?), reference_counts);
        assert_eq!(sorted(count_votes_08(&data)?), reference_counts);
        assert_eq!(sorted(count_votes_10(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_11(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_12(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_13(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_14(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_15(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_16(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_17(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_18(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_19(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_20(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_21(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_22(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_23(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_24(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_25(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_26(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_27(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_28(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_29(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_30(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_31(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_32(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_33(&data, &choices)?), reference_counts);
        assert_eq!(sorted(count_votes_34(&data, &choices)?), reference_counts);

        return Ok(());

    }
}