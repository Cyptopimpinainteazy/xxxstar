//! Free-space guard for authority nodes.
//!
//! Storage audit finding: X3 had no disk-full protection at all. Nothing
//! measured free space on the data volume, so the first symptom of a full disk
//! would be an `ENOSPC` returned from the middle of a trie write or a database
//! commit — the exact moment a validator must not be interrupted. The behaviour
//! the audit asks for is: notice the threshold, stop taking on work that cannot
//! be completed safely, and say so out loud, instead of discovering the problem
//! as a half-written commit and a mystery database state.
//!
//! Policy and measurement are split deliberately. [`classify`],
//! [`parse_min_free_bytes`] and [`parse_probe_interval`] are pure and unit
//! tested; [`free_bytes`] is the only part that touches the filesystem.

use std::path::Path;
use std::time::Duration;

/// Free-space floor for an authority node's data directory, in bytes.
pub const DEFAULT_MIN_FREE_BYTES: u64 = 1 << 30; // 1 GiB

/// Environment variable overriding [`DEFAULT_MIN_FREE_BYTES`].
///
/// Set it to `0` to switch the guard off entirely (for example inside a
/// deliberately tiny throwaway container).
pub const MIN_FREE_BYTES_ENV: &str = "X3_MIN_FREE_DISK_BYTES";

/// Environment variable for how often the watchdog re-measures, in seconds.
pub const PROBE_INTERVAL_ENV: &str = "X3_DISK_PROBE_SECS";

/// Default probe interval for the watchdog task.
pub const DEFAULT_PROBE_INTERVAL_SECS: u64 = 30;

/// Multiple of the floor at which the node starts warning rather than failing.
///
/// With the 1 GiB default this warns below 4 GiB, which is enough headroom for
/// an operator to reclaim space or migrate before authoring has to stop.
pub const WARNING_MULTIPLIER: u64 = 4;

/// How much free space is left relative to the configured floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskPressure {
    /// The guard is switched off (`min_free_bytes == 0`).
    Disabled,
    /// Comfortably above the warning band.
    Healthy,
    /// Inside the warning band, but still above the floor.
    Warning,
    /// At or below the floor: work that can no longer be completed safely.
    Critical,
}

impl DiskPressure {
    /// Whether the node may keep authoring at this pressure level.
    pub fn is_authoring_safe(self) -> bool {
        !matches!(self, DiskPressure::Critical)
    }
}

/// Classify free space against the configured floor.
pub fn classify(free_bytes: u64, min_free_bytes: u64) -> DiskPressure {
    if min_free_bytes == 0 {
        return DiskPressure::Disabled;
    }
    if free_bytes <= min_free_bytes {
        return DiskPressure::Critical;
    }
    if free_bytes <= min_free_bytes.saturating_mul(WARNING_MULTIPLIER) {
        return DiskPressure::Warning;
    }
    DiskPressure::Healthy
}

/// Resolve the configured floor, falling back to the default on garbage input.
///
/// An unparseable value is treated as "use the default" rather than "disable
/// the guard": a typo in one environment variable must not silently remove the
/// protection.
pub fn parse_min_free_bytes(raw: Option<&str>) -> u64 {
    let Some(raw) = raw else {
        return DEFAULT_MIN_FREE_BYTES;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_MIN_FREE_BYTES;
    }
    match trimmed.parse::<u64>() {
        Ok(bytes) => bytes,
        Err(_) => {
            log::warn!(
                "{MIN_FREE_BYTES_ENV}={trimmed:?} is not a byte count; using the default \
                 {DEFAULT_MIN_FREE_BYTES}"
            );
            DEFAULT_MIN_FREE_BYTES
        }
    }
}

/// Resolve the configured probe interval in seconds.
pub fn parse_probe_interval_secs(raw: Option<&str>) -> u64 {
    let Some(raw) = raw else {
        return DEFAULT_PROBE_INTERVAL_SECS;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_PROBE_INTERVAL_SECS;
    }
    match trimmed.parse::<u64>() {
        // A zero interval would spin; treat anything below one second as one.
        Ok(secs) => secs.max(1),
        Err(_) => {
            log::warn!(
                "{PROBE_INTERVAL_ENV}={trimmed:?} is not a whole number of seconds; using the \
                 default {DEFAULT_PROBE_INTERVAL_SECS}"
            );
            DEFAULT_PROBE_INTERVAL_SECS
        }
    }
}

/// Read the configured floor from the environment.
pub fn min_free_bytes_from_env() -> u64 {
    parse_min_free_bytes(std::env::var(MIN_FREE_BYTES_ENV).ok().as_deref())
}

/// Read the configured probe interval from the environment.
pub fn probe_interval_from_env() -> Duration {
    Duration::from_secs(parse_probe_interval_secs(
        std::env::var(PROBE_INTERVAL_ENV).ok().as_deref(),
    ))
}

/// Free bytes available to an unprivileged writer at `path`.
pub fn free_bytes(path: &Path) -> std::io::Result<u64> {
    fs2::available_space(path)
}

/// Operator-facing description of a pressure reading, if it needs saying.
pub fn describe(pressure: DiskPressure, free_bytes: u64, min_free_bytes: u64) -> Option<String> {
    match pressure {
        DiskPressure::Critical => Some(format!(
            "free disk space {free_bytes} bytes is at or below the {min_free_bytes} byte floor. \
             An authority must not start (or keep authoring) writes it cannot finish; reclaim \
             space or raise the volume, or set {MIN_FREE_BYTES_ENV}=0 to acknowledge the risk."
        )),
        DiskPressure::Warning => Some(format!(
            "free disk space {free_bytes} bytes is within {}x of the {min_free_bytes} byte floor; \
             authoring stops if it drops further.",
            WARNING_MULTIPLIER
        )),
        DiskPressure::Disabled => Some(format!(
            "{MIN_FREE_BYTES_ENV}=0 — disk-space protection is switched off for this node"
        )),
        DiskPressure::Healthy => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_flags_a_volume_inside_the_warning_band() {
        let floor = 1_000_000;
        assert_eq!(classify(floor, floor), DiskPressure::Critical);
        assert_eq!(classify(floor + 1, floor), DiskPressure::Warning);
        assert_eq!(
            classify(floor * WARNING_MULTIPLIER, floor),
            DiskPressure::Warning
        );
        assert_eq!(
            classify(floor * WARNING_MULTIPLIER + 1, floor),
            DiskPressure::Healthy
        );
    }

    #[test]
    fn classify_is_explicitly_disabled_by_a_zero_floor() {
        assert_eq!(classify(0, 0), DiskPressure::Disabled);
        assert_eq!(classify(u64::MAX, 0), DiskPressure::Disabled);
    }

    #[test]
    fn only_critical_pressure_stops_authoring() {
        assert!(!DiskPressure::Critical.is_authoring_safe());
        for safe in [
            DiskPressure::Healthy,
            DiskPressure::Warning,
            DiskPressure::Disabled,
        ] {
            assert!(safe.is_authoring_safe(), "{safe:?}");
        }
    }

    #[test]
    fn a_bad_floor_value_falls_back_to_the_default_instead_of_disabling() {
        assert_eq!(parse_min_free_bytes(None), DEFAULT_MIN_FREE_BYTES);
        assert_eq!(parse_min_free_bytes(Some("")), DEFAULT_MIN_FREE_BYTES);
        assert_eq!(parse_min_free_bytes(Some("twelve")), DEFAULT_MIN_FREE_BYTES);
        assert_eq!(
            parse_min_free_bytes(Some("not-a-number")),
            DEFAULT_MIN_FREE_BYTES
        );
    }

    #[test]
    fn an_explicit_floor_is_honoured_including_zero() {
        assert_eq!(parse_min_free_bytes(Some("0")), 0);
        assert_eq!(parse_min_free_bytes(Some(" 2048 ")), 2048);
    }

    #[test]
    fn probe_interval_never_spins() {
        assert_eq!(parse_probe_interval_secs(Some("0")), 1);
        assert_eq!(parse_probe_interval_secs(Some("15")), 15);
        assert_eq!(
            parse_probe_interval_secs(Some("nope")),
            DEFAULT_PROBE_INTERVAL_SECS
        );
        assert_eq!(parse_probe_interval_secs(None), DEFAULT_PROBE_INTERVAL_SECS);
    }

    #[test]
    fn a_healthy_reading_has_nothing_to_report() {
        assert!(describe(DiskPressure::Healthy, u64::MAX, 1 << 30).is_none());
    }

    #[test]
    fn the_critical_message_names_both_numbers_and_the_escape_hatch() {
        let text = describe(DiskPressure::Critical, 512, 1024).expect("critical must be reported");
        assert!(text.contains("512"), "{text}");
        assert!(text.contains("1024"), "{text}");
        assert!(text.contains(MIN_FREE_BYTES_ENV), "{text}");
    }

    #[test]
    fn free_bytes_reads_a_real_filesystem() {
        // The repository root always exists while the tests run; this asserts the
        // measurement path works, not any particular amount of space.
        let free = free_bytes(Path::new(".")).expect("the working directory must be measurable");
        assert!(free > 0, "a mounted filesystem reports some free space");
    }
}
