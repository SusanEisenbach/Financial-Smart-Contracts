use super::contract_combinator::{ Combinator, ContractCombinator, CombinatorDetails, deserialize_combinator, Box, Vec, Address };
use storage::*;
use { obs_values_key };

// The scale combinator
pub struct ScaleCombinator {
    // The sub-combinator
    sub_combinator: Box<ContractCombinator>,

    // The observable index
    obs_index: Option<usize>,

    // The scale value
    scale_value: Option<i64>,

    // The common combinator details
    combinator_details: CombinatorDetails
}

// Method implementation for the scale combinator
impl ScaleCombinator {
    pub fn new(sub_combinator: Box<ContractCombinator>, obs_index: Option<usize>, scale_value: Option<i64>) -> ScaleCombinator {
        if obs_index == None && scale_value == None {
            panic!("Scale combinator cannot be instantiated without a concrete observable index or scale value.");
        }

        ScaleCombinator {
            sub_combinator,
            obs_index,
            scale_value,
            combinator_details: CombinatorDetails::new()
        }
    }

    // Deserialize
    pub fn deserialize(index: usize, serialized_combinator: &Vec<i64>) -> (usize, Box<ContractCombinator>) {
        if index + 3 >= serialized_combinator.len() {
            panic!("Attempted to deserialize ill-formed serialized ScaleCombinator.")
        }
        let (index0, sub_combinator) = deserialize_combinator(index + 4, serialized_combinator);

        let mut obs_index: Option<usize> = None;
        let mut scale_value: Option<i64> = None;
        if serialized_combinator[index + 2] == 0 {
            obs_index = Some(serialized_combinator[index + 3] as usize);
        } else {
            scale_value = Some(serialized_combinator[index + 3]);
        }

        (
            index0,
            Box::new(ScaleCombinator {
                sub_combinator,
                obs_index,
                scale_value,
                combinator_details: CombinatorDetails::deserialize([serialized_combinator[index], serialized_combinator[index + 1]])
            })
        )
    }

    fn get_scale_value(&self, storage: &mut Storage) -> Option<i64> {
        match self.scale_value {
            Some(value) => Some(value),
            None => {
                match self.obs_index {
                    Some(index) => {
                        StoresFixedVec::<(Address, Option<i64>)>::get(storage, &obs_values_key(), index).1
                    },
                    None => panic!("Scale combinator has no scale value or observable index.")
                }
            }
        }
    }
}

// Contract combinator implementation for the scale combinator
impl ContractCombinator for ScaleCombinator {
    fn get_combinator_number(&self) -> Combinator {
        Combinator::SCALE
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
            panic!("Acquiring a previously-acquired scale combinator is not allowed.");
        }

        self.sub_combinator.acquire(time, storage);
        self.combinator_details.acquisition_time = Some(time);
    }

    // Updates the combinator, returning the current balance to be paid from the holder to the counter-party
    fn update(&mut self, time: u32, storage: &mut Storage) -> i64 {
        let scale_value = self.get_scale_value(storage);

        // If not acquired yet or fully updated (no more pending balance), return 0
        if self.combinator_details.acquisition_time == None
            || self.combinator_details.acquisition_time.unwrap() > time
            || self.combinator_details.fully_updated
            // If no scale value or obs value, don't update
            || scale_value == None {
            return 0;
        }

        let sub_value = self.sub_combinator.update(time, storage);
        self.combinator_details.fully_updated = self.sub_combinator.get_combinator_details().fully_updated;
        scale_value.unwrap() * sub_value
    }

    // Serializes this combinator
    fn serialize(&self) -> Vec<i64> {
        let mut serialized = self.serialize_details();

        // Store 0 then obs_index, or 1 then scale_value, depending on which exists
        match self.obs_index {
            Some(i) => {
                serialized.push(0);
                serialized.push(i as i64);
            },
            None => {
                serialized.push(1);
                serialized.push(self.scale_value.unwrap());
            }
        }

        serialized.extend_from_slice(&self.sub_combinator.serialize());
        serialized
    }
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::super::{ ContractCombinator, Combinator, ScaleCombinator, OneCombinator, TruncateCombinator };
    use super::super::contract_combinator::{ Address, Box, Vec, vec };
    use { obs_values_key };
    use storage::*;

    // Sets up the storage struct
    fn setup_storage(obs_values: &Vec<(Address, Option<i64>)>) -> Storage {
        let mut storage = Storage::new();
        storage.write_vec(&obs_values_key(), obs_values);
        storage
    }

    // Combinator number is correct
    #[test]
    fn correct_combinator_number() {
        let combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), Some(0), None);
        assert_eq!(combinator.get_combinator_number(), Combinator::SCALE);
    }

    // Horizon is equal to sub-combinator's horizon
    #[test]
    fn horizon_equals_sub_combinator_horizon() {
        // Create combinator scale 1 truncate 1 one
        let combinator = ScaleCombinator::new(
            Box::from(TruncateCombinator::new(
                Box::from(OneCombinator::new()),
                1
            )),
            None,
            Some(1)
        );

        // Check horizon
        let horizon = combinator.get_horizon();
        assert_eq!(
            horizon,
            Some(1),
            "Horizon of combinator 'scale 1 truncate 1 one' is not equal to Some(1): {:?}",
            horizon
        );
    }

    // Acquiring give-combinator sets combinator details correctly
    #[test]
    fn acquiring_sets_combinator_details() {
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Acquire and check details
        let time: u32 = 5;
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(time, &mut storage);
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
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Acquire and check value
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(0, &mut storage);
        let value = combinator.update(0, &mut storage);

        assert_eq!(
            value,
            5,
            "Update value of scale 5 one is not equal to 5: {}",
            value
        );
    }

    // Acquiring and updating combinator sets fully updated to true
    #[test]
    fn acquiring_and_updating_sets_fully_updated() {
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Acquire and check value
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(0, &mut storage);
        combinator.update(0, &mut storage);
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
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Acquire and check value
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(0, &mut storage);
        combinator.update(0, &mut storage);
        let value = combinator.update(0, &mut storage);

        assert_eq!(
            value,
            0,
            "Second update value of scale 5 one is not equal to 0: {}",
            value
        );
    }

    // Updating before acquiring does not set fully updated, and returns correct value
    #[test]
    fn updating_before_acquiring_does_nothing() {
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Update check details
        let mut storage = setup_storage(&vec![]);
        let value = combinator.update(0, &mut storage);
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
        // Create combinator scale 5 one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Update check details
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(1, &mut storage);
        let value = combinator.update(0, &mut storage);
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

    // Updating without concrete observable value does not set fully updated and returns correct value
    #[test]
    fn updating_without_concrete_observable_does_nothing() {
        // Create combinator scale obs one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), Some(0), None);

        // Update check details
        let mut storage = setup_storage(&vec![(Address::zero(), None)]);
        combinator.acquire(0, &mut storage);
        let value = combinator.update(0, &mut storage);
        let combinator_details = combinator.get_combinator_details();

        assert!(
            !combinator_details.fully_updated,
            "fully_updated != false: {}",
            combinator_details.fully_updated
        );

        assert_eq!(
            value,
            0,
            "Value of updating without concrete observable value != 0: {}",
            value
        )
    }

    // Updating with concrete observable value sets fully updated and returns correct value
    #[test]
    fn updating_with_concrete_observable_returns_correct_value() {
        // Create combinator scale obs one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), Some(0), None);

        // Update check details
        let mut storage = setup_storage(&vec![(Address::zero(), Some(10))]);
        combinator.acquire(0, &mut storage);
        let value = combinator.update(0, &mut storage);
        let combinator_details = combinator.get_combinator_details();

        assert_eq!(
            value,
            10,
            "Value of updating with concrete observable value != 10: {}",
            value
        );

        assert!(
            combinator_details.fully_updated,
            "fully_updated != true: {}",
            combinator_details.fully_updated
        );
    }

    // Updating without concrete observable value and then with a concrete value sets fully updated and returns correct value
    #[test]
    fn updating_twice_once_with_concrete_observable_returns_correct_value() {
        // Create combinator scale obs one
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), Some(0), None);

        // Update check details
        let mut storage = setup_storage(&vec![(Address::zero(), None)]);
        combinator.acquire(0, &mut storage);
        combinator.update(0, &mut storage);
        storage.set(&obs_values_key(), 0, (Address::zero(), Some(10 as i64)));
        let value = combinator.update(0, &mut storage);
        let combinator_details = combinator.get_combinator_details();

        assert_eq!(
            value,
            10,
            "Value of updating second time with concrete observable value != 10: {}",
            value
        );

        assert!(
            combinator_details.fully_updated,
            "fully_updated != true: {}",
            combinator_details.fully_updated
        );
    }

    // Serializing scale-combinator is correct when a scale value is set
    #[test]
    fn serialization_correct_scale_value() {
        let sub_combinator = OneCombinator::new();
        let sub_combinator_serialized = sub_combinator.serialize();
        let combinator = ScaleCombinator::new(Box::new(sub_combinator), Some(5), None);
        let serialized = combinator.serialize();
        assert_eq!(serialized[0..3], combinator.serialize_details()[..]);
        assert_eq!(serialized[3], 0);
        assert_eq!(serialized[4] as usize, combinator.obs_index.unwrap());
        assert_eq!(serialized[5..], sub_combinator_serialized[..]);
    }

    // Serializing scale-combinator is correct when an observable index is set
    #[test]
    fn serialization_correct_obs_index() {
        let sub_combinator = OneCombinator::new();
        let sub_combinator_serialized = sub_combinator.serialize();
        let combinator = ScaleCombinator::new(Box::new(sub_combinator), None, Some(5));
        let serialized = combinator.serialize();
        assert_eq!(serialized[0..3], combinator.serialize_details()[..]);
        assert_eq!(serialized[3], 1);
        assert_eq!(serialized[4], combinator.scale_value.unwrap());
        assert_eq!(serialized[5..], sub_combinator_serialized[..]);
    }

    // Deserializing scale-combinator is correct when a scale value is set
    #[test]
    fn deserialization_correct_scale_value() {
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(1, &mut storage);
        combinator.update(2, &mut storage);

        let serialized = combinator.serialize();
        let deserialized = ScaleCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized);
    }

    // Deserializing scale-combinator is correct when an observable index is set
    #[test]
    fn deserialization_correct_obs_index() {
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), Some(0), None);
        let mut storage = setup_storage(&vec![(Address::zero(), Some(10))]);
        combinator.acquire(1, &mut storage);
        combinator.update(2, &mut storage);

        let serialized = combinator.serialize();
        let deserialized = ScaleCombinator::deserialize(1, &serialized).1;
        assert_eq!(deserialized.serialize(), serialized);
    }
    
    // Scale combinator being instantiated without an observable index or scale value is not allowed
    #[test]
    #[should_panic(expected = "Scale combinator cannot be instantiated without a concrete observable index or scale value.")]
    fn should_panic_if_instantiated_without_obs_index_or_scale_value() {
        // Create combinator scale <> one
        ScaleCombinator::new(Box::from(OneCombinator::new()), None, None);
    }

    // Acquiring combinator twice is not allowed
    #[test]
    #[should_panic(expected = "Acquiring a previously-acquired scale combinator is not allowed.")]
    fn should_panic_when_acquiring_combinator_twice() {
        // Create combinator
        let mut combinator = ScaleCombinator::new(Box::new(OneCombinator::new()), None, Some(5));

        // Acquire twice
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(0, &mut storage);
        combinator.acquire(0, &mut storage);
    }

    // Acquiring combinator post-expiry is not allowed
    #[test]
    #[should_panic(expected = "Cannot acquire an expired contract.")]
    fn should_panic_when_acquiring_post_expiry() {
        // Create combinator
        let mut combinator = ScaleCombinator::new(
            Box::new(TruncateCombinator::new(
                Box::new(OneCombinator::new()),
                0
            )),
            None,
            Some(5)
        );

        // Acquire at time = 1
        let mut storage = setup_storage(&vec![]);
        combinator.acquire(1, &mut storage);
    }
}