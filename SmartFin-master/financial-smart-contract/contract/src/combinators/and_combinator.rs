use super::contract_combinator::{ Combinator, ContractCombinator, CombinatorDetails, latest_time, deserialize_combinator, Box, Vec };
use storage::Storage;

// The and combinator
pub struct AndCombinator {
    // The first sub-combinator
    sub_combinator0: Box<ContractCombinator>,

    // The second sub-combinator
    sub_combinator1: Box<ContractCombinator>,

    // The common combinator details
    combinator_details: CombinatorDetails
}

// Method implementation for the and combinator
impl AndCombinator {
    pub fn new(sub_combinator0: Box<ContractCombinator>, sub_combinator1: Box<ContractCombinator>) -> AndCombinator {
        AndCombinator {
            sub_combinator0,
            sub_combinator1,
            combinator_details: CombinatorDetails::new()
        }
    }

    // Deserialize
    pub fn deserialize(index: usize, serialized_combinator: &Vec<i64>) -> (usize, Box<ContractCombinator>) {
        if index + 1 >= serialized_combinator.len() {
            panic!("Attempted to deserialize ill-formed serialized AndCombinator.")
        }
        let (index0, sub_combinator0) = deserialize_combinator(index + 2, serialized_combinator);
        let (index1, sub_combinator1) = deserialize_combinator(index0, serialized_combinator);

        (
            index1,
            Box::new(AndCombinator {
                sub_combinator0,
                sub_combinator1,
                combinator_details: CombinatorDetails::deserialize([serialized_combinator[index], serialized_combinator[index + 1]])
            })
        )
    }
}

// Contract combinator implementation for the and combinator
impl ContractCombinator for AndCombinator {
    fn get_combinator_number(&self) -> Combinator {
        Combinator::AND
    }

    // Returns the latest of the two sub-horizons
    fn get_horizon(&self) -> Option<u32> {
        latest_time(self.sub_combinator0.get_horizon(), self.sub_combinator1.get_horizon())
    }

    fn get_combinator_details(&self) -> &CombinatorDetails {
        &self.combinator_details
    }

    // Acquires the combinator and acquirable sub-combinators
    fn acquire(&mut self, time: u32, storage: &mut Storage) {
        if self.past_horizon(time) {
            panic!("Cannot acquire an expired contract.");
        }
        if self.combinator_details.acquisition_time != None {
            panic!("Acquiring a previously-acquired and combinator is not allowed.");
        }

        if !self.sub_combinator0.past_horizon(time) {
            self.sub_combinator0.acquire(time, storage);
        }
        if !self.sub_combinator1.past_horizon(time) {
            self.sub_combinator1.acquire(time, storage);
        }
        self.combinator_details.acquisition_time = Some(time);
    }

    // Updates the combinator, returning the current balance to be paid from the holder to the counter-party
    fn update(&mut self, time: u32, storage: &mut Storage) -> i64 {
        // If not acquired yet or fully updated (no more pending balance), return 0
        if self.combinator_details.acquisition_time == None
            || self.combinator_details.acquisition_time.unwrap() > time
            || self.combinator_details.fully_updated {
            return 0;
        }

        let sub_value0 = self.sub_combinator0.update(time, storage);
        let sub_value1 = self.sub_combinator1.update(time, storage);
        self.combinator_details.fully_updated =
            self.sub_combinator0.get_combinator_details().fully_updated && self.sub_combinator1.get_combinator_details().fully_updated;
        sub_value0 + sub_value1
    }

    // Serializes this combinator
    fn serialize(&self) -> Vec<i64> {
        let mut serialized = self.serialize_details();
        serialized.extend_from_slice(&self.sub_combinator0.serialize());
        serialized.extend_from_slice(&self.sub_combinator1.serialize());
        serialized
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::super::{ ContractCombinator, Combinator, AndCombinator, OneCombinator, TruncateCombinator, ZeroCombinator };
    use super::super::contract_combinator::{ Box };
    use storage::Storage;

    // Combinator number is correct
    #[test]
    fn correct_combinator_number() {
        let combinator = AndCombinator::new(Box::new(OneCombinator::new()), Box::new(OneCombinator::new()));
        assert_eq!(combinator.get_combinator_number(), Combinator::AND);
    }
    
    // Horizon is latest of sub-combinators' horizons with the left combinator truncated
    #[test]
    fn correct_horizon_with_left_combinator_truncated() {
        // Create combinator and truncate 1 one one
        let combinator = AndCombinator::new(
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                1
            )),
            Box::from(OneCombinator::new())
        );

        // Check horizon == None
        let horizon = combinator.get_horizon();
        assert_eq!(
            horizon,
            None,
            "Value of 'and truncate 1 one one' contract is not equal to None: {:?}",
            horizon
        );
    }
    
    // Horizon is latest of sub-combinators' horizons with the right combinator truncated
    #[test]
    fn correct_horizon_with_right_combinator_truncated() {
        // Create combinator and one truncate 1 one
        let combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                1
            ))
        );

        // Check horizon == None
        let horizon = combinator.get_horizon();
        assert_eq!(
            horizon,
            None,
            "Value of 'and one truncate 1 one' contract is not equal to None: {:?}",
            horizon
        );
    }

    // Acquiring combinator sets combinator details correctly
    #[test]
    fn acquiring_sets_combinator_details() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Acquire and check details
        let time: u32 = 5;
        combinator.acquire(time, &mut Storage::new());
        let combinator_details = combinator.get_combinator_details();

        assert_eq!(
            combinator_details.acquisition_time,
            Some(time),
            "Acquisition time of combinator is not equal to Some(5): {:?}",
            combinator_details.acquisition_time
        );
    }

    // Acquiring and updating combinator returns correct value
    #[test]
    fn acquiring_and_updating_returns_correct_value() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            2,
            "Update value of and one one is not equal to 2: {}",
            value
        );
    }

    // Acquiring with one expired sub-combinator does not panic
    #[test]
    fn acquiring_with_one_expired_sub_combinator_should_not_panic() {
        // Create combinator and truncate 0 one one
        let mut combinator = AndCombinator::new(
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                0
            )),
            Box::from(OneCombinator::new())
        );

        // Acquire and check details
        let time: u32 = 5;
        combinator.acquire(time, &mut Storage::new());
    }

    // Acquiring and updating combinator sets fully updated to true
    #[test]
    fn acquiring_and_updating_sets_fully_updated() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        combinator.update(0, &mut Storage::new());
        let fully_updated = combinator.get_combinator_details().fully_updated;

        assert!(
            fully_updated,
            "fully_updated is not true: {}",
            fully_updated
        );
    }

    // Acquiring and updating combinator twice returns correct value
    #[test]
    fn acquiring_and_updating_twice_returns_correct_value() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        combinator.update(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            0,
            "Second update value of and one one is not equal to 0: {}",
            value
        );
    }

    // Updating before acquiring does not set fully updated, and returns correct value
    #[test]
    fn updating_before_acquiring_does_nothing() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Update check details
        let value = combinator.update(0, &mut Storage::new());
        let combinator_details = combinator.get_combinator_details();

        assert!(
            !combinator_details.fully_updated,
            "fully_updated != false: {}",
            combinator_details.fully_updated
        );

        assert_eq!(
            value,
            0,
            "Value of updating before acquiring != 0: {}",
            value
        )
    }

    // Updating before acquisition time does not set fully updated and returns correct value
    #[test]
    fn updating_before_acquisition_time_does_nothing() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Update check details
        combinator.acquire(1, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());
        let combinator_details = combinator.get_combinator_details();

        assert!(
            !combinator_details.fully_updated,
            "fully_updated != false: {}",
            combinator_details.fully_updated
        );

        assert_eq!(
            value,
            0,
            "Value of updating before acquiring != 0: {}",
            value
        )
    }

    // Serializing and-combinator is correct
    #[test]
    fn serialization_correct() {
        let sub_combinator0 = OneCombinator::new();
        let sub_combinator1 = ZeroCombinator::new();
        let mut sub_combinators_serialized = sub_combinator0.serialize();
        sub_combinators_serialized.extend_from_slice(&sub_combinator1.serialize()[..]);
        let combinator = AndCombinator::new(Box::new(sub_combinator0), Box::new(sub_combinator1));
        let serialized = combinator.serialize();
        assert_eq!(serialized[0..3], combinator.serialize_details()[..]);
        assert_eq!(serialized[3..], sub_combinators_serialized[..]);
    }

    // Deserializing and-combinator is correct
    #[test]
    fn deserialization_correct() {
        let mut combinator = AndCombinator::new(Box::new(OneCombinator::new()), Box::new(ZeroCombinator::new()));
        combinator.acquire(1, &mut Storage::new());
        combinator.update(2, &mut Storage::new());
        let serialized = combinator.serialize();
        let deserialized = AndCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized);
    }

    // Acquiring combinator twice is not allowed
    #[test]
    #[should_panic(expected = "Acquiring a previously-acquired and combinator is not allowed.")]
    fn should_panic_when_acquiring_combinator_twice() {
        // Create combinator and one one
        let mut combinator = AndCombinator::new(
            Box::from(OneCombinator::new()),
            Box::from(OneCombinator::new())
        );

        // Acquire twice
        combinator.acquire(0, &mut Storage::new());
        combinator.acquire(0, &mut Storage::new());
    }

    // Acquiring combinator post-expiry is not allowed
    #[test]
    #[should_panic(expected = "Cannot acquire an expired contract.")]
    fn should_panic_when_acquiring_post_expiry() {
        // Create combinator and truncate 0 one truncate 0 one
        let mut combinator = AndCombinator::new(
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                0
            )),
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                0
            ))
        );

        // Acquire at time = 1
        combinator.acquire(1, &mut Storage::new());
    }
}