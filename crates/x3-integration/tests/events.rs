//! `emit` reaches the receipt, and both engines journal the same events (X3-LANG-001).
//!
//! `emit` was refused at MIR lowering with "it has no lowering that preserves its meaning", and the
//! refusal was right about the tree: **neither** interpreter implemented opcode `0xA2`. The `std`
//! VM carried an `EventBuffer` that no instruction ever filled, the receipt's `logs` field was set
//! to `vec![]` beside a note saying collection was "deferred to runtime integration", and the
//! backend's encoder put the event's name in a constant-pool index the MIR lowering had no way to
//! produce. A program whose receipt is supposed to carry an event produced a receipt without one.
//!
//! These tests compile real `.x3` source and require the two engines — `crates/x3-vm`, which tools
//! and simulation reach, and `mini_x3`, which a block runs — to journal the same events for it.
#![cfg(all(feature = "std", feature = "compile"))]

use x3_x3_integration::compiler_bridge::compile_legacy_source as compile_source;
use x3_x3_integration::mini_x3;
use x3_x3_integration::{X3Executor, X3ExecutorConfig};

/// An event as both engines report it: the topic and the payload, which is what the receipt
/// carries.
type Journal = Vec<([u8; 32], Vec<u8>)>;

/// Compile `source`, run it on both engines, require them to agree, and return what they journalled.
fn agreed_events(source: &str) -> Journal {
    let bytes = compile_source(source).unwrap_or_else(|e| panic!("must compile: {e}\n{source}"));

    let receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
        .unwrap_or_else(|e| panic!("std engine: {e:?}\n{source}"));
    assert!(
        receipt.success,
        "std engine failed: {}\n{source}",
        String::from_utf8_lossy(&receipt.return_data)
    );
    let from_std: Journal = receipt
        .logs
        .iter()
        .map(|log| (log.topic.0, log.data.clone()))
        .collect();

    let from_mini: Journal = mini_x3::execute_x3bc(&bytes, 1_000_000)
        .unwrap_or_else(|e| panic!("mini_x3: {e:?}\n{source}"))
        .events
        .iter()
        .map(|log| (log.topic.0, log.data.clone()))
        .collect();

    assert_eq!(
        from_std, from_mini,
        "the engines journalled different events\n{source}"
    );

    // The on-chain entry point carries the same journal: this is the path a block takes.
    let on_chain = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false)
        .unwrap_or_else(|e| panic!("on-chain path: {e:?}\n{source}"));
    assert!(on_chain.success, "on-chain path failed\n{source}");
    let from_chain: Journal = on_chain
        .logs
        .iter()
        .map(|log| (log.topic.0, log.data.clone()))
        .collect();
    assert_eq!(
        from_chain, from_std,
        "the on-chain receipt differs\n{source}"
    );

    from_std
}

fn refusal(source: &str) -> String {
    match compile_source(source) {
        Ok(_) => panic!("must be refused at compile time:\n{source}"),
        Err(error) => error.to_string(),
    }
}

/// A tagged value, as an event's payload carries it: `[tag][len][data]`.
fn tagged(tag: u8, data: &[u8]) -> Vec<u8> {
    let mut out = vec![tag, data.len() as u8];
    out.extend_from_slice(data);
    out
}

fn topic_of(name: &str) -> [u8; 32] {
    sp_core::hashing::sha2_256(name.as_bytes())
}

#[test]
fn an_emitted_event_reaches_the_receipt_with_its_name_and_arguments() {
    let events = agreed_events("fn main() -> i64 { emit Transfer(1, 2); return 7; }");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].0,
        topic_of("Transfer"),
        "the topic is the name's hash"
    );
    let mut expected = tagged(x3_common::value_tags::INT, &1i64.to_le_bytes());
    expected.extend(tagged(x3_common::value_tags::INT, &2i64.to_le_bytes()));
    assert_eq!(
        events[0].1, expected,
        "the payload is the arguments, in order"
    );
}

/// Two events with different names must be distinguishable. Every event used to be named the
/// literal string "event" in the HIR lowering, so nothing that read a receipt could tell them
/// apart.
#[test]
fn different_events_have_different_topics() {
    let events = agreed_events(
        "fn main() -> i64 { emit Opened(1); emit Closed(1); emit Opened(2); return 0; }",
    );
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].0, topic_of("Opened"));
    assert_eq!(events[1].0, topic_of("Closed"));
    assert_eq!(events[2].0, topic_of("Opened"));
    assert_ne!(events[0].0, events[1].0, "two names, two topics");
    assert_eq!(
        events[0].0, events[2].0,
        "the same name is the same topic every time"
    );
    assert_ne!(
        events[0].1, events[2].1,
        "and the payloads still distinguish the two Opened events"
    );
}

#[test]
fn an_event_can_carry_no_arguments_or_arguments_of_each_kind() {
    assert_eq!(
        agreed_events("fn main() -> i64 { emit Ping(); return 0; }"),
        vec![(topic_of("Ping"), Vec::new())],
        "an event with no payload is still an event"
    );

    let events = agreed_events("fn main() -> i64 { emit Flag(true, 0 - 9); return 0; }");
    let mut expected = tagged(x3_common::value_tags::BOOL, &[1]);
    expected.extend(tagged(x3_common::value_tags::INT, &(-9i64).to_le_bytes()));
    assert_eq!(events[0].1, expected, "each argument carries its own tag");
}

/// Events are ordered and repeated exactly as the program emitted them: a loop that emits three
/// times journals three entries, in order, with the values of that iteration.
#[test]
fn a_loop_journals_one_event_per_iteration_in_order() {
    let events = agreed_events(
        "fn main() -> i64 { let mut i = 0; while i < 3 { emit Tick(i); i = i + 1; } return i; }",
    );
    assert_eq!(events.len(), 3);
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.0, topic_of("Tick"));
        assert_eq!(
            event.1,
            tagged(x3_common::value_tags::INT, &(index as i64).to_le_bytes()),
            "iteration {index}"
        );
    }
}

/// An `emit` whose result nobody reads must survive the optimizer: it is a call, and every pass
/// already treats a call as an effect. At `O0` the program is unoptimized, so the two runs also
/// show the optimizer changed nothing about what was emitted.
#[test]
fn the_optimizer_does_not_delete_an_event_nobody_reads() {
    let source = "fn main() -> i64 { emit Audited(42); return 0; }";
    let events = agreed_events(source);
    assert_eq!(events.len(), 1, "the event survived optimization");
    assert_eq!(
        events[0].1,
        tagged(x3_common::value_tags::INT, &42i64.to_le_bytes())
    );
}

/// A program that faults carries no events, the same rule its slot writes follow: a receipt
/// reports what survived, not what was in flight.
#[test]
fn a_faulted_program_journals_nothing() {
    let bytes =
        compile_source("fn main() -> i64 { emit Started(1); return 1 / 0; }").expect("compiles");
    let receipt = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).expect("runs");
    assert!(!receipt.success, "the program divides by zero");
    assert!(
        receipt.logs.is_empty(),
        "a faulted execution reports no events: {:?}",
        receipt.logs
    );
    assert!(mini_x3::execute_x3bc(&bytes, 1_000_000).is_err());
}

/// `emit` needs an event. `emit x;` names none, and the compiler refuses it rather than inventing
/// a name — which is what the lowering used to do for *every* event.
#[test]
fn emit_without_an_event_is_refused() {
    let error = refusal("fn main() -> i64 { let x = 1; emit x; return 0; }");
    assert!(error.contains("emit needs an event"), "{error}");
}
