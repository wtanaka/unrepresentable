use super::parse_utc;
use crate::failure::Failure;
use crate::types::{Count, Text};

macro_rules! epoch {
    ($text:literal) => {
        parse_utc(&Text::of($text))
    };
}

#[test]
fn the_epoch_is_zero() {
    assert_eq!(epoch!("1970-01-01T00:00:00Z"), Ok(Count::new(0)), "epoch");
}

#[test]
fn known_instants() {
    assert_eq!(
        epoch!("2000-03-01T00:00:00Z"),
        Ok(Count::new(951_868_800)),
        "after a century leap day rule"
    );
    assert_eq!(
        epoch!("2024-02-29T12:00:00Z"),
        Ok(Count::new(1_709_208_000)),
        "a leap day"
    );
    assert_eq!(
        epoch!("2024-01-31T23:59:59Z"),
        Ok(Count::new(1_706_745_599)),
        "end of January"
    );
}

#[test]
fn malformed_timestamps_are_refused() {
    for bad in [
        "",
        "2024-02-29 12:00:00Z",
        "2024-02-29T12:00:00",
        "2024-13-01T00:00:00Z",
        "2024-02-29T25:00:00Z",
        "1969-12-31T23:59:59Z",
        "2024-02-2xT12:00:00Z",
    ] {
        let result: Result<Count, Failure> = parse_utc(&Text::of(bad));
        assert!(result.is_err(), "{bad:?} must be refused");
    }
}
