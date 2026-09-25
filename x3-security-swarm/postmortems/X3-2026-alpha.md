# Incident Postmortem — X3-2026-α

This fictional postmortem defines the reporting standard for real incidents. It is written as a containment success, not a hero narrative.

## Summary

A coordinated flash-loan probe attempted to exploit oracle latency across two contracts attached to the cross-VM path. The attempt was contained before any asset loss or irreversible settlement occurred.

## Timeline

At `T+0s`, `Sentinel-Watcher` flagged abnormal gas timing, path rehearsal behavior, and wallet clustering patterns consistent with a staged exploit. At `T+12s`, `Sentinel-Judge` raised the attribution score to the high-confidence band after correlating timing signatures with a known exploit family. At `T+18s`, `Sentinel-Warden` reached quorum and paused the two affected contracts while rate-limiting the suspect wallet segment. At `T+30s`, `Sentinel-Scribe` finalized the incident bundle, wrote the evidence package to immutable storage, and anchored the bundle hash.

## Impact

No funds were lost. Two contracts were paused for nine minutes. User-facing latency increased during the containment window because the affected path was routed to a safer fallback.

## Root cause

The initiating weakness was excessive oracle tolerance during a high-load period, which left too much room for a time-sensitive exploit rehearsal to mature.

## Corrective actions

Oracle tolerance thresholds were tightened, the honeypot pattern library was updated to mirror the observed exploit sequence, and judge calibration was adjusted to reduce hesitation around similar multi-signal probes.

## Outcome

Containment succeeded, the false-positive review concluded that the response was proportionate, and no permanent sanctions were issued.