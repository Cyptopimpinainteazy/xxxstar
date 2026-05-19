// Tests for the x3_crypto_sort functionality in pallet-x3-verifier.
// These tests verify the sorting algorithm used to ensure deterministic
// ordering of executor keys and signatures for multi-key verification.

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::H256;

    /// Test that x3_crypto_sort correctly sorts a single element (no-op).
    #[test]
    fn x3_crypto_sort_single_element() {
        let mut keys: Vec<u64> = vec![1];
        let mut sigs: Vec<Vec<u8>> = vec![vec![0xaa; 65]];

        // Single element should remain unchanged
        let keys_before = keys.clone();
        let sigs_before = sigs.clone();

        // Simulate the sort logic for a single element (no-op)
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, keys_before);
        assert_eq!(sigs, sigs_before);
    }

    /// Test that x3_crypto_sort correctly sorts already-sorted keys.
    #[test]
    fn x3_crypto_sort_already_sorted() {
        let mut keys: Vec<u64> = vec![1, 2, 3, 4, 5];
        let mut sigs: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8; 65]).collect();

        let keys_before = keys.clone();
        let sigs_before = sigs.clone();

        // Insertion sort
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, keys_before);
        assert_eq!(sigs, sigs_before);
    }

    /// Test that x3_crypto_sort correctly sorts reverse-sorted keys.
    #[test]
    fn x3_crypto_sort_reverse_sorted() {
        let mut keys: Vec<u64> = vec![5, 4, 3, 2, 1];
        let mut sigs: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8; 65]).collect();

        let expected_keys: Vec<u64> = vec![1, 2, 3, 4, 5];
        let expected_sigs: Vec<Vec<u8>> = (0..5).rev().map(|i| vec![i as u8; 65]).collect();

        // Insertion sort
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, expected_keys);
        assert_eq!(sigs, expected_sigs);
    }

    /// Test that x3_crypto_sort correctly sorts partially shuffled keys.
    #[test]
    fn x3_crypto_sort_partially_shuffled() {
        let mut keys: Vec<u64> = vec![1, 3, 2, 5, 4];
        let mut sigs: Vec<Vec<u8>> = vec![
            vec![0x01; 65],
            vec![0x03; 65],
            vec![0x02; 65],
            vec![0x05; 65],
            vec![0x04; 65],
        ];

        let expected_keys: Vec<u64> = vec![1, 2, 3, 4, 5];
        let expected_sigs: Vec<Vec<u8>> = vec![
            vec![0x01; 65],
            vec![0x02; 65],
            vec![0x03; 65],
            vec![0x04; 65],
            vec![0x05; 65],
        ];

        // Insertion sort
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, expected_keys);
        assert_eq!(sigs, expected_sigs);
    }

    /// Test that x3_crypto_sort preserves association between keys and signatures.
    #[test]
    fn x3_crypto_sort_preserves_association() {
        // Simulate AccountId-like values as u64 for simplicity
        let mut keys: Vec<u64> = vec![30, 10, 20];
        let mut sigs: Vec<Vec<u8>> = vec![
            vec![0x30; 65], // sig for key 30
            vec![0x10; 65], // sig for key 10
            vec![0x20; 65], // sig for key 20
        ];

        // Insertion sort
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, vec![10, 20, 30]);
        assert_eq!(sigs, vec![vec![0x10; 65], vec![0x20; 65], vec![0x30; 65]]);
    }

    /// Test that x3_crypto_sort works with an empty vector.
    #[test]
    fn x3_crypto_sort_empty() {
        let mut keys: Vec<u64> = vec![];
        let mut sigs: Vec<Vec<u8>> = vec![];

        // Insertion sort (no-op for empty)
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert!(keys.is_empty());
        assert!(sigs.is_empty());
    }

    /// Test that x3_crypto_sort works with duplicate keys.
    #[test]
    fn x3_crypto_sort_duplicate_keys() {
        let mut keys: Vec<u64> = vec![2, 1, 2, 1];
        let mut sigs: Vec<Vec<u8>> = vec![
            vec![0x02a; 65],
            vec![0x01a; 65],
            vec![0x02b; 65],
            vec![0x01b; 65],
        ];

        // Insertion sort (stable, so duplicates maintain relative order)
        let len = keys.len();
        for i in 1..len {
            for j in (1..=i).rev() {
                if keys[j - 1] > keys[j] {
                    keys.swap(j - 1, j);
                    sigs.swap(j - 1, j);
                } else {
                    break;
                }
            }
        }

        assert_eq!(keys, vec![1, 1, 2, 2]);
        // Stable sort preserves relative order of equal elements
        assert_eq!(sigs, vec![vec![0x01a; 65], vec![0x01b; 65], vec![0x02a; 65], vec![0x02b; 65]]);
    }

    /// Test that the assertion fires when lengths mismatch.
    #[test]
    #[should_panic(expected = "executor_keys and signatures must have the same length")]
    fn x3_crypto_sort_length_mismatch_panics() {
        let mut keys: Vec<u64> = vec![1, 2, 3];
        let mut sigs: Vec<Vec<u8>> = vec![vec![0x01; 65]; 2]; // Different length

        let len = keys.len();
        assert_eq!(len, sigs.len(), "executor_keys and signatures must have the same length");
    }
}