use super::contract_combinator::{ Combinator, ContractCombinator, CombinatorDetails, Vec, Box };
use storage::Storage;

// The one combinator
pub struct OneCombinator {
    // The common combinator details
    combinator_details: CombinatorDetails
}

// Method implementation of the one combinator
impl OneCombinator {
    // Constructor
    pub fn new() -> OneCombinator {
        OneCombinator {
            combinator_details: CombinatorDetails::new()
        }
    }

    // Deserialize
    pub fn deserialize(index: usize, serialized_combinator: &Vec<i64>) -> (usize, Box<ContractCombinator>) {
        if index + 1 >= serialized_combinator.len() {
            panic!("Attempted to deserialize ill-formed serialized OneCombinator.")
        }
        (
            index + 2,
            Box::new(OneCombinator {
                combinator_details: CombinatorDetails::deserialize([serialized_combinator[index], serialized_combinator[index + 1]])
            })
        )
    }
}

// Contract combinator implementation of the one combinator
impl ContractCombinator for OneCombinator {
    fn get_combinator_number(&self) -> Combinator {
        Combinator::ONE
    }

    fn get_combinator_details(&self) -> &CombinatorDetails {
        &self.combinator_details
    }

    // Acquires the combinator and acquirable sub-combinators
    fn acquire(&mut self, time: u32, _: &mut Storage) {
        if self.combinator_details.acquisition_time != None {
            panic!("Acquiring a previously-acquired one combinator is not allowed.");
        }

        self.combinator_details.acquisition_time = Some(time);
    }

    // Updates the combinator, returning the current balance to be paid from the holder to the counter-party
    fn update(&mut self, time: u32, _: &mut Storage) -> i64 {
        // If not acquired yet or fully updated (no more pending balance), return 0
        if self.combinator_details.acquisition_time == None
            || self.combinator_details.acquisition_time.unwrap() > time
            || self.combinator_details.fully_updated {
            return 0;
        }

        self.combinator_details.fully_updated = true;
        1
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::super::{ ContractCombinator, Combinator, OneCombinator };
    use storage::Storage;

    // Combinator number is correct
    #[test]
    fn correct_combinator_number() {
        let combinator = OneCombinator::new();
        assert_eq!(combinator.get_combinator_number(), Combinator::ONE);
    }

    // Horizon is None
    #[test]
    fn correct_horizon() {
        // Create combinator one
        let combinator = OneCombinator::new();

        // Check horizon = None
        let horizon = combinator.get_horizon();
        assert_eq!(
            horizon,
            None,
            "Value of 'one' contract is not equal to None: {:?}",
            horizon
        );
    }

    // Acquiring combinator sets combinator details correctly
    #[test]
    fn acquiring_sets_combinator_details() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

        // Acquire and check details
        let time: u32 = 5;
        combinator.acquire(time, &mut Storage::new());
        let combinator_details = combinator.get_combinator_details();

        assert_eq!(
            combinator_details.acquisition_time,
            Some(time),
            "Acquisition time of one combinator is not equal to Some(5): {:?}",
            combinator_details.acquisition_time
        );
    }

    // Acquiring and updating combinator sets fully updated to 
    #[test]
    fn acquiring_and_updating_sets_fully_updated() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

        // Acquire and check details
        let time: u32 = 5;
        combinator.acquire(time, &mut Storage::new());
        combinator.update(time, &mut Storage::new());
        let combinator_details = combinator.get_combinator_details();

        assert!(
            combinator_details.fully_updated,
            "not fully_updated: {}",
            combinator_details.fully_updated
        );
    }

    // Acquiring and updating combinator returns correct value
    #[test]
    fn acquiring_and_updating_returns_correct_value() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            1,
            "Acquisition value of one combinator is not equal to 1: {}",
            value
        );
    }

    // Acquiring and updating combinator twice returns correct value
    #[test]
    fn acquiring_and_updating_twice_returns_correct_value() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

        // Acquire and check value
        combinator.acquire(0, &mut Storage::new());
        combinator.update(0, &mut Storage::new());
        let value = combinator.update(0, &mut Storage::new());

        assert_eq!(
            value,
            0,
            "Value of one combinator after acquisition is not equal to 0: {}",
            value
        );
    }

    // Updating before acquiring does not set fully updated, and returns correct value
    #[test]
    fn updating_before_acquiring_does_nothing() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

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
        // Create combinator one
        let mut combinator = OneCombinator::new();

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

    // Deserializing one-combinator is correct
    #[test]
    fn deserialization_correct() {
        let mut combinator = OneCombinator::new();
        let mut serialized = combinator.serialize();
        let mut deserialized = OneCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized);

        combinator.acquire(1, &mut Storage::new());
        combinator.update(2, &mut Storage::new());
        serialized = combinator.serialize();
        deserialized = OneCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized)
    }

    // Acquiring one-combinator twice is not allowed
    #[test]
    #[should_panic(expected = "Acquiring a previously-acquired one combinator is not allowed.")]
    fn should_panic_when_acquiring_one_combinator_twice() {
        // Create combinator one
        let mut combinator = OneCombinator::new();

        // Acquire twice
        combinator.acquire(0, &mut Storage::new());
        combinator.acquire(0, &mut Storage::new());
    }
}