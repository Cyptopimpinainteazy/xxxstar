//! Volunteer GPU protocol boundary.
//!
//! This module deliberately contains no shell execution, network listener,
//! blockchain extrinsics, or GPU driver calls. It is a typed validation layer
//! for a future Inferstructor adapter.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolunteerPolicy {
    pub consent: bool,
    pub revoked: bool,
    pub max_vram_mib: u32,
    pub max_job_seconds: u32,
    pub max_daily_seconds: u32,
    pub allow_benchmark: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolunteerJob {
    pub id: String,
    pub requested_vram_mib: u32,
    pub duration_seconds: u32,
    pub benchmark: bool,
    pub paid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    NoConsent,
    Revoked,
    UnsupportedWorkload,
    PaidJob,
    InvalidDuration,
    MemoryLimit,
    TimeLimit,
    DailyLimit,
}

pub fn authorize(
    policy: &VolunteerPolicy,
    job: &VolunteerJob,
    consumed_today_seconds: u32,
) -> Result<(), Rejection> {
    if !policy.consent { return Err(Rejection::NoConsent); }
    if policy.revoked { return Err(Rejection::Revoked); }
    if job.paid { return Err(Rejection::PaidJob); }
    if !job.benchmark || !policy.allow_benchmark {
        return Err(Rejection::UnsupportedWorkload);
    }
    if job.duration_seconds == 0 { return Err(Rejection::InvalidDuration); }
    if job.requested_vram_mib > policy.max_vram_mib {
        return Err(Rejection::MemoryLimit);
    }
    if job.duration_seconds > policy.max_job_seconds {
        return Err(Rejection::TimeLimit);
    }
    if job.duration_seconds > policy.max_daily_seconds.saturating_sub(consumed_today_seconds) {
        return Err(Rejection::DailyLimit);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> VolunteerPolicy {
        VolunteerPolicy { consent: true, revoked: false, max_vram_mib: 8192,
            max_job_seconds: 300, max_daily_seconds: 600, allow_benchmark: true }
    }
    fn job() -> VolunteerJob {
        VolunteerJob { id: "test".into(), requested_vram_mib: 4096,
            duration_seconds: 60, benchmark: true, paid: false }
    }
    #[test] fn accepts_bounded_volunteer_benchmark() {
        assert_eq!(authorize(&policy(), &job(), 0), Ok(()));
    }
    #[test] fn rejects_no_consent_and_revocation() {
        let mut p = policy(); p.consent = false;
        assert_eq!(authorize(&p, &job(), 0), Err(Rejection::NoConsent));
        p.consent = true; p.revoked = true;
        assert_eq!(authorize(&p, &job(), 0), Err(Rejection::Revoked));
    }
    #[test] fn rejects_paid_and_unsupported_jobs() {
        let mut j = job(); j.paid = true;
        assert_eq!(authorize(&policy(), &j, 0), Err(Rejection::PaidJob));
        j.paid = false; j.benchmark = false;
        assert_eq!(authorize(&policy(), &j, 0), Err(Rejection::UnsupportedWorkload));
    }
    #[test] fn enforces_resource_limits() {
        let mut j = job(); j.requested_vram_mib = 9000;
        assert_eq!(authorize(&policy(), &j, 0), Err(Rejection::MemoryLimit));
        j = job(); j.duration_seconds = 301;
        assert_eq!(authorize(&policy(), &j, 0), Err(Rejection::TimeLimit));
        j.duration_seconds = 60;
        assert_eq!(authorize(&policy(), &j, 550), Err(Rejection::DailyLimit));
        j.duration_seconds = 0;
        assert_eq!(authorize(&policy(), &j, 0), Err(Rejection::InvalidDuration));
    }
}
