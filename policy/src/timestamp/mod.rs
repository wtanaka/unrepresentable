//! UTC timestamps as seconds since the Unix epoch, from the one format the
//! GitHub API uses: `YYYY-MM-DDTHH:MM:SSZ`.

#[cfg(test)]
mod tests;

use crate::failure::Failure;
use crate::types::{Count, Message, Text};

/// The number of seconds in a day.
const SECONDS_PER_DAY: usize = 86_400;
/// The number of seconds in an hour.
const SECONDS_PER_HOUR: usize = 3_600;
/// The number of seconds in a minute.
const SECONDS_PER_MINUTE: usize = 60;
/// The number of days in a 400-year era.
const DAYS_PER_ERA: usize = 146_097;
/// The number of days from year 0 to 1970.
const DAYS_BEFORE_1970: usize = 719_468;
/// The length of a timestamp such as `2024-01-02T03:04:05Z`.
const FORMAT_LENGTH: usize = 20;

/// A numeric field of the timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// The day of the month.
    Day,
    /// The hour.
    Hour,
    /// The minute.
    Minute,
    /// The month.
    Month,
    /// The second.
    Second,
    /// The year.
    Year,
}

impl Part {
    /// Reads this field from `text`.
    #[must_use]
    fn read(self, text: &Text) -> Option<Count> {
        let (from, length) = self.span();
        let end = from.get().checked_add(length.get())?;
        Count::parse(&Text::of(text.as_str().get(from.get()..end)?))
    }

    /// Where the field starts and how many digits it has.
    const fn span(self) -> (Count, Count) {
        match self {
            Self::Year => (Count::new(0), Count::new(4)),
            Self::Month => (Count::new(5), Count::new(2)),
            Self::Day => (Count::new(8), Count::new(2)),
            Self::Hour => (Count::new(11), Count::new(2)),
            Self::Minute => (Count::new(14), Count::new(2)),
            Self::Second => (Count::new(17), Count::new(2)),
        }
    }
}

/// A calendar date.
struct Civil {
    /// The day of the month.
    day: Count,
    /// The month.
    month: Count,
    /// The year.
    year: Count,
}

/// Days from 1970-01-01 to the given civil date (proleptic Gregorian).
fn days_from_civil(date: &Civil) -> Result<Count, Failure> {
    let one = Count::new(1);
    let adjusted_year = if date.month.get() <= 2 {
        date.year.minus(one)?
    } else {
        date.year
    };
    let era = adjusted_year.div(Count::new(400))?;
    let year_of_era = adjusted_year.minus(era.times(Count::new(400))?)?;
    let month_index = if date.month.get() > 2 {
        date.month.minus(Count::new(3))?
    } else {
        date.month.plus(Count::new(9))?
    };
    let day_of_year = month_index
        .times(Count::new(153))?
        .plus(Count::new(2))?
        .div(Count::new(5))?
        .plus(date.day)?
        .minus(one)?;
    let day_of_era = year_of_era
        .times(Count::new(365))?
        .plus(year_of_era.div(Count::new(4))?)?
        .minus(year_of_era.div(Count::new(100))?)?
        .plus(day_of_year)?;
    era.times(Count::new(DAYS_PER_ERA))?
        .plus(day_of_era)?
        .minus(Count::new(DAYS_BEFORE_1970))
}

/// Seconds since the epoch for a `YYYY-MM-DDTHH:MM:SSZ` timestamp.
pub(crate) fn parse_utc(text: &Text) -> Result<Count, Failure> {
    let bad = || {
        Failure::parse(&Message::from_string(format!(
            "bad timestamp {:?}",
            text.as_str()
        )))
    };
    let found = text.as_str();
    let shaped = found.len() == FORMAT_LENGTH
        && found.get(4..5) == Some("-")
        && found.get(7..8) == Some("-")
        && found.get(10..11) == Some("T")
        && found.get(13..14) == Some(":")
        && found.get(16..17) == Some(":")
        && found.get(19..20) == Some("Z");
    if !shaped {
        return Err(bad());
    }
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        Part::Year.read(text),
        Part::Month.read(text),
        Part::Day.read(text),
        Part::Hour.read(text),
        Part::Minute.read(text),
        Part::Second.read(text),
    ) else {
        return Err(bad());
    };
    if year.get() < 1970 || month.get() < 1 || month.get() > 12 || day.get() < 1 || day.get() > 31 {
        return Err(bad());
    }
    if hour.get() > 23 || minute.get() > 59 || second.get() > 60 {
        return Err(bad());
    }
    days_from_civil(&Civil { day, month, year })?
        .times(Count::new(SECONDS_PER_DAY))?
        .plus(hour.times(Count::new(SECONDS_PER_HOUR))?)?
        .plus(minute.times(Count::new(SECONDS_PER_MINUTE))?)?
        .plus(second)
}
