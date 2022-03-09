//! Helper function for writing tests

use std::{collections::HashSet, iter};

/// Generate an iterator of bytestring keys, with increasing length up to a
/// maximum value.
///
/// This iterator will produce `max_len` number of keys. Each key has the form
/// `[0*, u8::MAX]`, meaning zero or more 0 values, followed by a single
/// `u8::MAX` value. The final `u8::MAX` value is added to ensure that no key is
/// a prefix of another key generated by this function.
///
/// # Examples
///
/// ```
/// # use blart::tests_common::generate_keys_skewed;
/// let mut keys = generate_keys_skewed(10).collect::<Vec<_>>();
/// assert_eq!(keys.len(), 10);
/// assert_eq!(keys[0].as_ref(), &[255]);
/// assert_eq!(keys[keys.len() - 1].as_ref(), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 255]);
///
/// for k in keys {
///     println!("{:?}", k);
/// }
/// ```
///
/// The above example will print
/// ```text
/// [255]
/// [0, 255]
/// [0, 0, 255]
/// [0, 0, 0, 255]
/// [0, 0, 0, 0, 255]
/// [0, 0, 0, 0, 0, 255]
/// [0, 0, 0, 0, 0, 0, 255]
/// [0, 0, 0, 0, 0, 0, 0, 255]
/// [0, 0, 0, 0, 0, 0, 0, 0, 255]
/// [0, 0, 0, 0, 0, 0, 0, 0, 0, 255]
/// ```
///
/// # Panics
///  - Panics if `max_len` is 0.
pub fn generate_keys_skewed(max_len: usize) -> impl Iterator<Item = Box<[u8]>> {
    assert!(max_len > 0, "the fixed key length must be greater than 0");

    iter::successors(Some(vec![u8::MAX; 1].into_boxed_slice()), move |prev| {
        if prev.len() < max_len {
            let mut key = vec![u8::MIN; prev.len()];
            key.push(u8::MAX);
            Some(key.into_boxed_slice())
        } else {
            None
        }
    })
}

/// Generate an iterator of bytestring keys, all with the same length.
///
/// This iterator will produce `(value_stops + 1) ^ max_len` keys in total. The
/// keys produced by this iterator will every string from the alphabet `{ 0 *
/// (255 / value_stops), 1 * (255 / value_stops), ..., 255 }` of length
/// `max_len`.
///
/// # Examples
///
/// ```
/// # use blart::tests_common::generate_key_fixed_length;
/// let mut keys = generate_key_fixed_length(3, 2).collect::<Vec<_>>();
/// assert_eq!(keys.len(), 27);
/// assert_eq!(keys[0].as_ref(), &[0, 0, 0]);
/// assert_eq!(keys[keys.len() / 2].as_ref(), &[128, 128, 128]);
/// assert_eq!(keys[keys.len() - 1].as_ref(), &[255, 255, 255]);
///
/// for k in keys {
///     println!("{:?}", k);
/// }
/// ```
///
/// The above example will print
/// ```text
/// [0, 0, 0]
/// [0, 0, 128]
/// [0, 0, 255]
/// [0, 128, 0]
/// [0, 128, 128]
/// [0, 128, 255]
/// [0, 255, 0]
/// [0, 255, 128]
/// [0, 255, 255]
/// [128, 0, 0]
/// [128, 0, 128]
/// [128, 0, 255]
/// [128, 128, 0]
/// [128, 128, 128]
/// [128, 128, 255]
/// [128, 255, 0]
/// [128, 255, 128]
/// [128, 255, 255]
/// [255, 0, 0]
/// [255, 0, 128]
/// [255, 0, 255]
/// [255, 128, 0]
/// [255, 128, 128]
/// [255, 128, 255]
/// [255, 255, 0]
/// [255, 255, 128]
/// [255, 255, 255]
/// ```
///
/// # Panics
///
///  - Panics if `max_len` is 0.
///  - Panics if `value_stops` is 0.
pub fn generate_key_fixed_length(
    max_len: usize,
    value_stops: u8,
) -> impl Iterator<Item = Box<[u8]>> {
    struct FixedLengthKeys {
        increment: u8,
        next_value: Option<Box<[u8]>>,
    }

    impl FixedLengthKeys {
        pub fn new(max_len: usize, value_stops: u8) -> Self {
            assert!(max_len > 0, "the fixed key length must be greater than 0");
            assert!(
                value_stops > 0,
                "the number of distinct values for each key digit must be greater than 0"
            );

            fn div_ceil(lhs: u8, rhs: u8) -> u8 {
                let d = lhs / rhs;
                let r = lhs % rhs;
                if r > 0 && rhs > 0 {
                    d + 1
                } else {
                    d
                }
            }

            let increment = div_ceil(u8::MAX, value_stops);

            FixedLengthKeys {
                increment,
                next_value: Some(vec![u8::MIN; max_len].into_boxed_slice()),
            }
        }
    }

    impl Iterator for FixedLengthKeys {
        type Item = Box<[u8]>;

        fn next(&mut self) -> Option<Self::Item> {
            let next_value = self.next_value.take()?;

            if next_value.iter().all(|digit| *digit == u8::MAX) {
                // the .take function already updated the next_value to None
                return Some(next_value);
            }

            let mut new_next_value = next_value.clone();
            for idx in (0..new_next_value.len()).rev() {
                if new_next_value[idx] == u8::MAX {
                    new_next_value[idx] = u8::MIN;
                } else {
                    new_next_value[idx] = new_next_value[idx].saturating_add(self.increment);
                    break;
                }
            }

            self.next_value = Some(new_next_value);
            Some(next_value)
        }
    }

    FixedLengthKeys::new(max_len, value_stops)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixExpansion {
    pub base_index: usize,
    pub expanded_length: usize,
}

/// Generate an iterator of fixed length bytestring keys, where specific
/// portions of the key are expanded as duplicate bytes.
///
/// This is meant to simulate keys with shared prefixes in different portions of
/// the key string.
///
/// # Examples
///
/// ```
/// # use blart::tests_common::{generate_key_with_prefix, PrefixExpansion};
/// let mut keys = generate_key_with_prefix(3, 2, [PrefixExpansion { base_index: 0, expanded_length: 3 }]).collect::<Vec<_>>();
/// assert_eq!(keys.len(), 27);
/// assert_eq!(keys[0].as_ref(), &[0, 0, 0, 0, 0]);
/// assert_eq!(keys[(keys.len() / 2) - 2].as_ref(), &[128, 128, 128, 0, 255]);
/// assert_eq!(keys[keys.len() - 1].as_ref(), &[255, 255, 255, 255, 255]);
///
/// for k in keys {
///     println!("{:?}", k);
/// }
/// ```
///
/// The above example will print out:
/// ```text
/// [0, 0, 0, 0, 0]
/// [0, 0, 0, 0, 128]
/// [0, 0, 0, 0, 255]
/// [0, 0, 0, 128, 0]
/// [0, 0, 0, 128, 128]
/// [0, 0, 0, 128, 255]
/// [0, 0, 0, 255, 0]
/// [0, 0, 0, 255, 128]
/// [0, 0, 0, 255, 255]
/// [128, 128, 128, 0, 0]
/// [128, 128, 128, 0, 128]
/// [128, 128, 128, 0, 255]
/// [128, 128, 128, 128, 0]
/// [128, 128, 128, 128, 128]
/// [128, 128, 128, 128, 255]
/// [128, 128, 128, 255, 0]
/// [128, 128, 128, 255, 128]
/// [128, 128, 128, 255, 255]
/// [255, 255, 255, 0, 0]
/// [255, 255, 255, 0, 128]
/// [255, 255, 255, 0, 255]
/// [255, 255, 255, 128, 0]
/// [255, 255, 255, 128, 128]
/// [255, 255, 255, 128, 255]
/// [255, 255, 255, 255, 0]
/// [255, 255, 255, 255, 128]
/// [255, 255, 255, 255, 255]
/// ```
///
/// # Panics
///
///  - Panics if `base_key_len` is 0.
///  - Panics if `value_stops` is 0.
///  - Panics if any PrefixExpansion has `expanded_length` equal to 0.
///  - Panics if any PrefixExpansion has `base_index` greater than or equal to
///    `base_key_len`.
pub fn generate_key_with_prefix(
    base_key_len: usize,
    value_stops: u8,
    prefix_expansions: impl AsRef<[PrefixExpansion]>,
) -> impl Iterator<Item = Box<[u8]>> {
    let expansions = prefix_expansions.as_ref();

    assert!(
        expansions
            .iter()
            .all(|expand| { expand.base_index < base_key_len }),
        "the prefix expansion index must be less than `base_key_len`."
    );
    assert!(
        expansions
            .iter()
            .all(|expand| { expand.expanded_length > 0 }),
        "the prefix expansion length must be greater than 0."
    );
    {
        let mut uniq_indices = HashSet::new();
        assert!(
            expansions
                .iter()
                .all(|expand| uniq_indices.insert(expand.base_index)),
            "the prefix expansion index must be unique"
        );
    }

    let mut sorted_expansions = expansions.to_vec();
    sorted_expansions.sort_by(|a, b| a.base_index.cmp(&b.base_index));

    let full_key_len = expansions
        .iter()
        .map(|expand| expand.expanded_length - 1)
        .sum::<usize>()
        + base_key_len;
    let full_key_template = vec![u8::MIN; full_key_len].into_boxed_slice();

    fn apply_expansions_to_key(
        old_key: &[u8],
        new_key_template: &[u8],
        sorted_expansions: &[PrefixExpansion],
    ) -> Box<[u8]> {
        let mut new_key: Box<[u8]> = new_key_template.into();
        let mut new_key_index = 0usize;
        let mut old_key_index = 0usize;

        for expansion in sorted_expansions {
            let before_len = expansion.base_index - old_key_index;
            new_key[new_key_index..(new_key_index + before_len)]
                .copy_from_slice(&old_key[old_key_index..expansion.base_index]);
            new_key[(new_key_index + before_len)
                ..(new_key_index + before_len + expansion.expanded_length)]
                .fill(old_key[expansion.base_index]);

            old_key_index = expansion.base_index + 1;
            new_key_index += before_len + expansion.expanded_length
        }

        // copy over remaining bytes from the old_key
        new_key[new_key_index..].copy_from_slice(&old_key[old_key_index..]);

        new_key
    }

    generate_key_fixed_length(base_key_len, value_stops)
        .map(move |key| apply_expansions_to_key(&key, &full_key_template, &sorted_expansions))
}
