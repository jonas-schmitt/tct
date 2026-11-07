//! Runtime Modulo Behavior Tests
//!
//! These tests verify that modulo-based graceful recovery actually works at runtime,
//! not just in generated code patterns. Tests decode actual out-of-range tokens
//! and verify correct wraparound behavior.

use tct::tokenizer::JsonTokenEncoder;

#[test]
fn test_bool_modulo_runtime_behavior() {
    // Bool has 2 values: false (0), true (1)
    // Test that modulo works: token % 2

    // Normal cases
    let mut iter = vec![0u32].into_iter();
    assert!(!bool::decode(&mut iter).unwrap());

    let mut iter = vec![1u32].into_iter();
    assert!(bool::decode(&mut iter).unwrap());

    // Modulo cases - out of range tokens should wrap
    let mut iter = vec![2u32].into_iter(); // 2 % 2 = 0 -> false
    assert!(!bool::decode(&mut iter).unwrap());

    let mut iter = vec![3u32].into_iter(); // 3 % 2 = 1 -> true
    assert!(bool::decode(&mut iter).unwrap());

    let mut iter = vec![100u32].into_iter(); // 100 % 2 = 0 -> false
    assert!(!bool::decode(&mut iter).unwrap());

    let mut iter = vec![101u32].into_iter(); // 101 % 2 = 1 -> true
    assert!(bool::decode(&mut iter).unwrap());

    let mut iter = vec![999u32].into_iter(); // 999 % 2 = 1 -> true
    assert!(bool::decode(&mut iter).unwrap());
}

#[test]
fn test_option_modulo_runtime_behavior() {
    // Option<T> uses OptionalField which has modulo logic
    // 0 = Absent, 1 = Null, 2 = Present(T)
    // But for Option: 0 = None, 1 = Some

    // Normal cases
    let mut iter = vec![0u32].into_iter();
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), None);

    // i64 uses 8-byte little-endian encoding: 42i64 = [42, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![1u32, 42, 0, 0, 0, 0, 0, 0, 0].into_iter();
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), Some(42));

    // Modulo cases
    let mut iter = vec![2u32].into_iter(); // 2 % 2 = 0 -> None
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), None);

    // 123i64 = [123, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![3u32, 123, 0, 0, 0, 0, 0, 0, 0].into_iter(); // 3 % 2 = 1 -> Some
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), Some(123));

    let mut iter = vec![100u32].into_iter(); // 100 % 2 = 0 -> None
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), None);

    // 77i64 = [77, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![999u32, 77, 0, 0, 0, 0, 0, 0, 0].into_iter(); // 999 % 2 = 1 -> Some
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), Some(77));
}

#[test]
fn test_optional_field_modulo_runtime_behavior() {
    use tct::tokenizer::optional_field::OptionalField;

    // OptionalField has 3 states: Absent (0), Null (1), Present (2)

    // Normal cases
    let mut iter = vec![0u32].into_iter();
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Absent
    );

    let mut iter = vec![1u32].into_iter();
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Null
    );

    // i64 uses 8-byte little-endian encoding: 42i64 = [42, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![2u32, 42, 0, 0, 0, 0, 0, 0, 0].into_iter();
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Present(42)
    );

    // Modulo cases - token % 3
    let mut iter = vec![3u32].into_iter(); // 3 % 3 = 0 -> Absent
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Absent
    );

    let mut iter = vec![4u32].into_iter(); // 4 % 3 = 1 -> Null
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Null
    );

    // 99i64 = [99, 0, 0, 0, 0, 0, 0, 0]
    let mut iter = vec![5u32, 99, 0, 0, 0, 0, 0, 0, 0].into_iter(); // 5 % 3 = 2 -> Present
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Present(99)
    );

    let mut iter = vec![99u32].into_iter(); // 99 % 3 = 0 -> Absent
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Absent
    );

    let mut iter = vec![100u32].into_iter(); // 100 % 3 = 1 -> Null
    assert_eq!(
        OptionalField::<i64>::decode(&mut iter).unwrap(),
        OptionalField::Null
    );
}

#[test]
fn test_bpe_decoder_modulo_runtime_behavior() {
    use tct::bpe::encoder::BpeDecoder;
    use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};

    // Create test merge table
    // base_vocab_size = 80
    // Merges: [5,12]->80, [80,7]->81
    // max_vocab_size = 100
    let table = BpeMergeTable {
        schema: "test".to_string(),
        base_vocab_size: 80,
        merges: vec![
            BpeMerge {
                pair: [5, 12],
                new_token: 80,
                frequency: 1000,
            },
            BpeMerge {
                pair: [80, 7],
                new_token: 81,
                frequency: 500,
            },
        ],
        metadata: BpeMetadata {
            trained_on_instances: 100,
            training_date: "2025-10-25".to_string(),
            max_merges: 2,
            dict_size: None,
        },
    };

    let decoder = BpeDecoder::new(&table, 100);

    // Test 1: Normal base tokens pass through
    let decoded = decoder.decode(&[5, 12, 7]);
    assert_eq!(decoded, vec![5, 12, 7]);

    // Test 2: Valid BPE tokens expand correctly
    let decoded = decoder.decode(&[80]); // [5,12]->80
    assert_eq!(decoded, vec![5, 12]);

    let decoded = decoder.decode(&[81]); // [80,7]->81 -> [5,12,7]
    assert_eq!(decoded, vec![5, 12, 7]);

    // Test 3: Out-of-range tokens (>= max_vocab_size) normalize via modulo
    let decoded = decoder.decode(&[150]); // 150 % 100 = 50 (base token)
    assert_eq!(decoded, vec![50]);

    let decoded = decoder.decode(&[999]); // 999 % 100 = 99 (BPE range, not in table) -> 99 % 80 = 19
    assert_eq!(decoded, vec![19]);

    // Test 4: Unknown BPE tokens (in BPE range but not in merge table)
    // Token 85 is in [80..100) but not in table
    // Should map to base: 85 % 80 = 5
    let decoded = decoder.decode(&[85]);
    assert_eq!(decoded, vec![5]);

    // Token 99 is in [80..100) but not in table
    // Should map to base: 99 % 80 = 19
    let decoded = decoder.decode(&[99]);
    assert_eq!(decoded, vec![19]);
}

#[test]
fn test_bpe_decoder_boundary_conditions() {
    use tct::bpe::encoder::BpeDecoder;
    use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};

    let table = BpeMergeTable {
        schema: "boundary".to_string(),
        base_vocab_size: 80,
        merges: vec![BpeMerge {
            pair: [0, 1],
            new_token: 80,
            frequency: 100,
        }],
        metadata: BpeMetadata {
            trained_on_instances: 10,
            training_date: "2025-10-25".to_string(),
            max_merges: 1,
            dict_size: None,
        },
    };

    let decoder = BpeDecoder::new(&table, 100);

    // Boundary: token = max_vocab_size - 1 (valid, highest token)
    let decoded = decoder.decode(&[99]); // 99 % 100 = 99 (in BPE range, not in table)
    assert_eq!(decoded, vec![19]); // 99 % 80 = 19

    // Boundary: token = max_vocab_size (wraps to 0)
    let decoded = decoder.decode(&[100]); // 100 % 100 = 0 (base token)
    assert_eq!(decoded, vec![0]);

    // Boundary: token = base_vocab_size (first BPE token, but may not be in table)
    let decoded = decoder.decode(&[80]); // Valid BPE token
    assert_eq!(decoded, vec![0, 1]); // Expands correctly

    // Boundary: token = base_vocab_size - 1 (last base token)
    let decoded = decoder.decode(&[79]); // Base token
    assert_eq!(decoded, vec![79]);

    // Boundary: token = u32::MAX (extreme value)
    let decoded = decoder.decode(&[u32::MAX]); // u32::MAX % 100 = 95
    assert_eq!(decoded, vec![15]); // 95 % 80 = 15
}

#[test]
fn test_bpe_decoder_base_vocab_start() {
    use tct::bpe::encoder::BpeDecoder;
    use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};

    let table = BpeMergeTable {
        schema: "string-bpe".to_string(),
        base_vocab_size: 257,
        merges: vec![BpeMerge {
            pair: [97, 98],
            new_token: 257,
            frequency: 4,
        }],
        metadata: BpeMetadata {
            trained_on_instances: 4,
            training_date: "2026-04-24".to_string(),
            max_merges: 1,
            dict_size: None,
        },
    };

    let decoder = BpeDecoder::new(&table, 260);

    let decoded = decoder.decode(&[256]);
    assert_eq!(
        decoded,
        vec![256],
        "base token 256 must not be treated as BPE"
    );

    let decoded = decoder.decode(&[257]);
    assert_eq!(decoded, vec![97, 98]);

    let decoded = decoder.decode(&[259]);
    assert_eq!(
        decoded,
        vec![2],
        "unknown BPE-range tokens should normalize against BPE_START"
    );
}

#[test]
fn test_surplus_token_detection_with_buffered_iterator() {
    use tct::tokenizer::{BufferedTokenIterator, JsonTokenEncoder};

    // Test 1: Exact token count (no surplus)
    let tokens = vec![0u32]; // bool: false
    let total = tokens.len();
    let mut iter = BufferedTokenIterator::new(tokens.into_iter());
    let value = bool::decode(&mut iter).unwrap();
    assert!(!value);
    assert_eq!(iter.remaining(), 0); // All tokens consumed
    let consumed = total - iter.remaining();
    assert_eq!(consumed, 1);

    // Test 2: Surplus tokens
    let tokens = vec![1u32, 999u32, 100u32]; // bool: true, then 2 surplus tokens
    let total = tokens.len();
    let mut iter = BufferedTokenIterator::new(tokens.into_iter());
    let value = bool::decode(&mut iter).unwrap();
    assert!(value);
    assert_eq!(iter.remaining(), 2, "Should have 2 surplus tokens");
    let consumed = total - iter.remaining();
    assert_eq!(consumed, 1); // Only 1 token consumed for bool

    // Test 3: Option with surplus
    let tokens = vec![0u32, 42u32]; // None, then surplus token
    let total = tokens.len();
    let mut iter = BufferedTokenIterator::new(tokens.into_iter());
    let value = Option::<i64>::decode(&mut iter).unwrap();
    assert_eq!(value, None);
    assert_eq!(iter.remaining(), 1, "Should have 1 surplus token");
    let consumed = total - iter.remaining();
    assert_eq!(consumed, 1); // Option::None consumes only 1 token
}

#[test]
fn test_extreme_boundary_values() {
    use tct::tokenizer::JsonTokenEncoder;

    // Test 1: u32::MAX with bool (largest possible token)
    let mut iter = vec![u32::MAX].into_iter();
    let result = bool::decode(&mut iter).unwrap();
    assert!(result); // u32::MAX % 2 = 1 -> true

    // Test 2: Zero values
    let mut iter = vec![0u32].into_iter();
    assert!(!bool::decode(&mut iter).unwrap());

    let mut iter = vec![0u32].into_iter();
    assert_eq!(Option::<i64>::decode(&mut iter).unwrap(), None);

    // Test 3: Power-of-2 boundaries for bool (modulo 2)
    let mut iter = vec![2u32].into_iter(); // Exact modulo boundary
    assert!(!bool::decode(&mut iter).unwrap()); // 2 % 2 = 0

    let mut iter = vec![256u32].into_iter(); // Power of 2
    assert!(!bool::decode(&mut iter).unwrap()); // 256 % 2 = 0

    let mut iter = vec![65536u32].into_iter(); // Larger power of 2
    assert!(!bool::decode(&mut iter).unwrap()); // 65536 % 2 = 0

    // Test 4: One less than boundaries
    let mut iter = vec![1u32].into_iter();
    assert!(bool::decode(&mut iter).unwrap()); // 1 % 2 = 1

    let mut iter = vec![255u32].into_iter();
    assert!(bool::decode(&mut iter).unwrap()); // 255 % 2 = 1

    let mut iter = vec![65535u32].into_iter();
    assert!(bool::decode(&mut iter).unwrap()); // 65535 % 2 = 1
}

#[test]
fn test_nested_modulo_integration() {
    use tct::bpe::encoder::BpeDecoder;
    use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
    use tct::tokenizer::JsonTokenEncoder;
    use tct::tokenizer::optional_field::OptionalField;

    // Test nested structures where multiple modulo operations interact:
    // - OptionalField modulo 3
    // - Inner bool modulo 2
    // - BPE decoder modulo

    // Test 1: OptionalField::Present with out-of-range bool
    let mut iter = vec![2u32, 100u32, 0u32].into_iter(); // 2%3=2 (Present), 100%2=0 (false)
    let result = OptionalField::<bool>::decode(&mut iter).unwrap();
    assert_eq!(result, OptionalField::Present(false));

    // Test 2: OptionalField with extreme variant index
    let mut iter = vec![999u32, 101u32, 0u32].into_iter(); // 999%3=0 (Absent)
    let result = OptionalField::<bool>::decode(&mut iter).unwrap();
    assert_eq!(result, OptionalField::Absent); // Absent doesn't consume remaining tokens

    // Test 3: BPE decoder with chained modulo operations
    let table = BpeMergeTable {
        schema: "nested".to_string(),
        base_vocab_size: 10,
        merges: vec![BpeMerge {
            pair: [1, 2],
            new_token: 10,
            frequency: 100,
        }],
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2025-10-25".to_string(),
            max_merges: 1,
            dict_size: None,
        },
    };

    let decoder = BpeDecoder::new(&table, 20);

    // Token 999: 999 % 20 = 19 (BPE range but not in table) -> 19 % 10 = 9
    let decoded = decoder.decode(&[999]);
    assert_eq!(decoded, vec![9]);

    // Token u32::MAX: u32::MAX % 20 = 15 (BPE range) -> 15 % 10 = 5
    let decoded = decoder.decode(&[u32::MAX]);
    assert_eq!(decoded, vec![5]);
}
