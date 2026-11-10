use foxmorrow_core::Error;
use foxmorrow_core::model::{Kind, Outcome, Status, VerificationState};
use foxmorrow_core::util::{new_run_id, sha256_hex};

#[test]
fn sha256_matches_the_standard_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn run_ids_are_short_and_unique() {
    let first = new_run_id();
    let second = new_run_id();
    assert!(first.starts_with("r-"));
    assert_eq!(first.len(), 12);
    assert_ne!(first, second);
}

#[test]
fn statuses_round_trip_through_their_names() {
    for kind in [Kind::Author, Kind::Review, Kind::Qa, Kind::Root] {
        assert_eq!(Kind::parse(kind.as_str()).unwrap(), kind);
    }
    assert_eq!(Status::parse("interrupted").unwrap(), Status::Interrupted);
    assert_eq!(Outcome::parse("unverified").unwrap(), Outcome::Unverified);
    assert_eq!(
        VerificationState::parse("rejected").unwrap(),
        VerificationState::Rejected
    );
    assert_eq!(
        serde_json::to_string(&Outcome::Passed).unwrap(),
        "\"passed\""
    );
}

#[test]
fn unknown_names_are_invalid() {
    assert!(matches!(
        Kind::parse("orchestrator"),
        Err(Error::Invalid(_))
    ));
}
