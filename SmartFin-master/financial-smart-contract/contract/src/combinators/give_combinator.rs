use super::contract_combinator::{ Combinator, ContractCombinator, CombinatorDetails, deserialize_combinator, Box, Vec };
use storage::Storage;

// The give combinator
pub struct GiveCombinator {
    // The sub-combinator
    sub_combinator: Box<ContractCombinator>,

    // The common combinator details
    combinator_details: CombinatorDetails
}

// Method implementation for the give combinator
impl GiveCombinator {
    pub fn new(sub_combinator: Box<ContractCombinator>) -> GiveCombinator {
        GiveCombinator {
            sub_combinator,
            combinator_details: CombinatorDetails::new()
        }
    }

    // Deserialize
    pub fn deserialize(index: usize, serialized_combinator: &Vec<i64>) -> (usize, Box<ContractCombinator>) {
        if index + 1 >= serialized_combinator.len() {
            panic!("Attempted to deserialize ill-formed serialized GiveCombinator.")
        }
        let (index0, sub_combinator) = deserialize_combinator(index + 2, serialized_combinator);

        (
            index0,
            Box::new(GiveCombinator {
                sub_combinator,
                combinator_details: CombinatorDetails::deserialize([serialized_combinator[index], serialized_combinator[index + 1]])
            })
        )
    }
}

// Contract combinator implementation for the give combinator
impl ContractCombinator for GiveCombinator {
    fn get_combinator_number(&self) -> Combinator {
        Combinator::GIVE
    }

    fn get_horizon(&self) -> Option<u32> {
        self.sub_combinator.get_horizon()
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
            panic!("Acquiring a previously-acquired give combinator is not allowed.");
        }

        self.sub_combinator.acquire(time, storage);
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

        let sub_value = self.sub_combinator.update(time, storage);
        self.combinator_details.fully_updated = self.sub_combinator.get_combinator_details().fully_updated;
        -1 * sub_value
    }

    // Serializes this combinator
    fn serialize(&self) -> Vec<i64> {
        let mut serialized = self.serialize_details();
        serialized.extend_from_slice(&self.sub_combinator.serialize());
        serialized
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::super::{ ContractCombinator, Combinator, GiveCombinator, OneCombinator, TruncateCombinator };
    use super::super::contract_combinator::{ Box };
    use storage::Storage;

    // Combinator number is correct
    #[test]
    fn correct_combinator_number() {
        let combinator = GiveCombinator::new(Box::new(OneCombinator::new()));
        assert_eq!(combinator.get_combinator_number(), Combinator::GIVE);
    }

    // Horizon is equal to sub-combinator's horizon
    #[test]
    fn horizon_equals_sub_combinator_horizon() {
        // Create combinator give truncate 1 one
        let combinator = GiveCombinator::new(
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                1
            ))
        );

        // Check horizon
        let horizon = combinator.get_horizon();
        assert_eq!(
            horizon,
            Some(1),
            "Horizon of combinator 'give truncate 1 one' is not equal to Some(1): {:?}",
            horizon
        );
    }

    // Acquiring give-combinator sets combinator details correctly
    #[test]
    fn acquiring_sets_combinator_details() {
        // Create combinator give one
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

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
        // Create combinator
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            -1,
            "Update value of give one is not equal to -1: {}",
            value
        );
    }

    // Acquiring and updating combinator sets fully updated to true
    #[test]
    fn acquiring_and_updating_sets_fully_updated() {
        // Create combinator
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

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
        // Create combinator
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        combinator.update(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            0,
            "Second update value of give one is not equal to 0: {}",
            value
        );
    }

    // Updating before acquiring does not set fully updated, and returns correct value
    #[test]
    fn updating_before_acquiring_does_nothing() {
        // Create combinator give one
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

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
        // Create combinator give one
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

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

    // Serializing give-combinator is correct
    #[test]
    fn serialization_correct() {
        let sub_combinator = OneCombinator::new();
        let sub_combinator_serialized = sub_combinator.serialize();
        let combinator = GiveCombinator::new(Box::new(sub_combinator));
        let serialized = combinator.serialize();
        assert_eq!(serialized[0..3], combinator.serialize_details()[..]);
        assert_eq!(serialized[3..], sub_combinator_serialized[..]);
    }

    // Deserializing give-combinator is correct
    #[test]
    fn deserialization_correct() {
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));
        combinator.acquire(1, &mut Storage::new());
        combinator.update(2, &mut Storage::new());
        let serialized = combinator.serialize();
        let deserialized = GiveCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized);
    }

    // Acquiring combinator twice is not allowed
    #[test]
    #[should_panic(expected = "Acquiring a previously-acquired give combinator is not allowed.")]
    fn should_panic_when_acquiring_combinator_twice() {
        // Create combinator
        let mut combinator = GiveCombinator::new(Box::new(OneCombinator::new()));

        // Acquire twice
        combinator.acquire(0, &mut Storage::new());
        combinator.acquire(0, &mut Storage::new());
    }

    // Acquiring combinator post-expiry is not allowed
    #[test]
    #[should_panic(expected = "Cannot acquire an expired contract.")]
    fn should_panic_when_acquiring_post_expiry() {
        // Create combinator
        let mut combinator = GiveCombinator::new(
            Box::new(TruncateCombinator::new(
                Box::new(OneCombinator::new()),
                0
            ))
        );

        // Acquire at time = 1
        combinator.acquire(1, &mut Storage::new());
    }
}