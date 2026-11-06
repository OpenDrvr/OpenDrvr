//! Small, dependency-free date and timestamp validation.
//!
//! Catalog imports are offline tooling input, so malformed dates should be
//! rejected with a useful message rather than silently normalized. The checks
//! cover the RFC 3339 subset used by the catalog: calendar dates and UTC or
//! offset timestamps.

/// Returns true for a valid `YYYY-MM-DD` calendar date.
pub fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    let Some(year) = parse_digits(&bytes[0..4]) else {
        return false;
    };
    let Some(month) = parse_digits(&bytes[5..7]) else {
        return false;
    };
    let Some(day) = parse_digits(&bytes[8..10]) else {
        return false;
    };

    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }

    day >= 1 && day <= days_in_month(year, month)
}

/// Returns true for a valid RFC 3339 timestamp with a required offset.
///
/// Examples: `2026-11-06T12:34:56Z` and
/// `2026-11-06T12:34:56.123+02:00`.
pub fn is_rfc3339_timestamp(value: &str) -> bool {
    let Some((date, rest)) = value.split_once('T').or_else(|| value.split_once('t')) else {
        return false;
    };

    if !is_iso_date(date) {
        return false;
    }

    let (time, offset) =
        if let Some(time) = rest.strip_suffix('Z').or_else(|| rest.strip_suffix('z')) {
            (time, "Z")
        } else if let Some(index) = rest.rfind('+').or_else(|| rest.rfind('-')) {
            (&rest[..index], &rest[index..])
        } else {
            return false;
        };

    is_clock_time(time) && is_offset(offset)
}

fn is_clock_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(hours), Some(minutes), Some(seconds), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };

    let Some(hours) = parse_digits(hours.as_bytes()) else {
        return false;
    };
    let Some(minutes) = parse_digits(minutes.as_bytes()) else {
        return false;
    };

    let (seconds, fraction) = match seconds.split_once('.') {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (seconds, None),
    };

    let Some(seconds) = parse_digits(seconds.as_bytes()) else {
        return false;
    };

    if hours > 23 || minutes > 59 || seconds > 60 {
        return false;
    }

    match fraction {
        Some(fraction) => {
            !fraction.is_empty()
                && fraction.len() <= 9
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => true,
    }
}

fn is_offset(value: &str) -> bool {
    if value.eq_ignore_ascii_case("Z") {
        return true;
    }

    let bytes = value.as_bytes();
    if bytes.len() != 6 || (bytes[0] != b'+' && bytes[0] != b'-') || bytes[3] != b':' {
        return false;
    }

    let Some(hours) = parse_digits(&bytes[1..3]) else {
        return false;
    };
    let Some(minutes) = parse_digits(&bytes[4..6]) else {
        return false;
    };

    hours <= 23 && minutes <= 59
}

fn parse_digits(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.iter().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    bytes.iter().try_fold(0_u32, |value, byte| {
        value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))
    })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_dates() {
        assert!(is_iso_date("2026-11-06"));
        assert!(is_iso_date("2024-02-29"));
        assert!(is_iso_date("2000-02-29"));
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(!is_iso_date("2026-11-31"));
        assert!(!is_iso_date("2025-02-29"));
        assert!(!is_iso_date("1900-02-29"));
        assert!(!is_iso_date("2026-13-01"));
        assert!(!is_iso_date("2026-00-01"));
        assert!(!is_iso_date("2026-1-01"));
        assert!(!is_iso_date("2026/11/06"));
        assert!(!is_iso_date(""));
    }

    #[test]
    fn accepts_valid_timestamps() {
        assert!(is_rfc3339_timestamp("2026-11-06T12:34:56Z"));
        assert!(is_rfc3339_timestamp("2026-11-06t12:34:56z"));
        assert!(is_rfc3339_timestamp("2026-11-06T12:34:56.123456789Z"));
        assert!(is_rfc3339_timestamp("2026-11-06T12:34:56+02:00"));
        assert!(is_rfc3339_timestamp("2026-11-06T12:34:56-05:30"));
        assert!(is_rfc3339_timestamp("2026-12-31T23:59:60Z"));
    }

    #[test]
    fn rejects_invalid_timestamps() {
        assert!(!is_rfc3339_timestamp("2026-11-06T12:34:56"));
        assert!(!is_rfc3339_timestamp("2026-11-06T24:00:00Z"));
        assert!(!is_rfc3339_timestamp("2026-11-06T12:60:00Z"));
        assert!(!is_rfc3339_timestamp("2026-11-06T12:34:61Z"));
        assert!(!is_rfc3339_timestamp("2026-11-06T12:34:56.Z"));
        assert!(!is_rfc3339_timestamp("2026-11-06T12:34:56+24:00"));
        assert!(!is_rfc3339_timestamp("2026-11-06T12:34:56+02"));
        assert!(!is_rfc3339_timestamp("not-a-timestamp"));
    }
}
