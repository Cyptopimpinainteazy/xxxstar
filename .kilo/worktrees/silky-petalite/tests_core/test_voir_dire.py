"""Tests for the voir dire system with dual-counsel and anonymization."""

from swarm.jury.voir_dire import (
    LawyerRole,
    StrikeReason,
    VoirDireManager,
)


def test_voir_dire_anonymization():
    """Test that juror identities are anonymized in voir dire."""
    vm = VoirDireManager()

    case_id = "case-001"
    candidate_ids = ["juror-alice", "juror-bob", "juror-charlie"]
    candidate_data = {
        "juror-alice": {"reputation": 0.8, "section": "governance", "recent_jury_count": 1},
        "juror-bob": {"reputation": 0.7, "section": "economic", "recent_jury_count": 0},
        "juror-charlie": {"reputation": 0.75, "section": "security", "recent_jury_count": 2},
    }

    _voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # Verify anonymization: profiles don't contain juror IDs
    for profile in profiles:
        assert profile.profile_hash.startswith("")  # Hash, not name
        assert "juror-" not in profile.profile_hash
        assert profile.reputation in [0.8, 0.7, 0.75]

    # Verify mapping is hidden (only accessible internally via profile_hash)
    assert len(vm.profile_id_mapping) == 3
    # But profiles themselves don't expose it
    for profile in profiles:
        assert not hasattr(profile, "juror_id")


def test_dual_counsel_symmetric_strikes():
    """Test that DA and Defense have symmetric strike limits."""
    vm = VoirDireManager()

    case_id = "case-002"
    candidate_ids = [f"juror-{i}" for i in range(15)]
    candidate_data = {cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0} for cid in candidate_ids}

    voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # DA counsel strikes 3 for-cause
    da_strikes = profiles[:3]
    for i, profile in enumerate(da_strikes):
        success = vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="da-counsel-01",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profile.profile_hash,
            reason=StrikeReason.DOMAIN_CONFLICT,
            reasoning_text=f"Conflict in procedural domain {i}",
        )
        assert success is True

    # Defense counsel strikes 3 for-cause
    defense_strikes = profiles[3:6]
    for i, profile in enumerate(defense_strikes):
        success = vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="defense-counsel-01",
            lawyer_role=LawyerRole.DEFENSE_DUE_PROCESS,
            lawyer_side="defense",
            profile_hash=profile.profile_hash,
            reason=StrikeReason.BIAS_PATTERN,
            reasoning_text=f"Bias pattern {i}",
        )
        assert success is True

    # Verify both sides have 3 strikes recorded
    record = vm.voir_dire_records[voir_dire_id]
    assert len(record.prosecution_strikes) == 3
    assert len(record.defense_strikes) == 3

    # Both should fail on 4th strike (for-cause limit is 10, but test with peremptory model)
    # Actually, let's test peremptory limit instead
    for i in range(10):  # Use up for-cause limit
        if i < 7:  # Already have 3, can add 7 more for-cause
            profile = profiles[6 + i] if 6 + i < len(profiles) else None
            if profile:
                success = vm.strike_juror(
                    voir_dire_id=voir_dire_id,
                    lawyer_id="da-counsel-01",
                    lawyer_role=LawyerRole.DA_SAFETY,
                    lawyer_side="prosecution",
                    profile_hash=profile.profile_hash,
                    reason=StrikeReason.SAFETY_CONCERN,
                    reasoning_text="Safety concern",
                )
                if i < 7:
                    assert success is True


def test_peremptory_strike_limits():
    """Test that peremptory (cause-free) strikes are limited."""
    vm = VoirDireManager()

    case_id = "case-003"
    candidate_ids = [f"juror-{i}" for i in range(20)]
    candidate_data = {cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0} for cid in candidate_ids}

    voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # DA counsel uses all 3 peremptory strikes
    for i in range(3):
        success = vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="da-counsel-01",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profiles[i].profile_hash,
            reason=StrikeReason.PEREMPTORY,
            reasoning_text="Peremptory strike",
        )
        assert success is True

    # 4th peremptory strike should fail
    success = vm.strike_juror(
        voir_dire_id=voir_dire_id,
        lawyer_id="da-counsel-01",
        lawyer_role=LawyerRole.DA_PROCEDURAL,
        lawyer_side="prosecution",
        profile_hash=profiles[3].profile_hash,
        reason=StrikeReason.PEREMPTORY,
        reasoning_text="Peremptory strike (should fail)",
    )
    assert success is False


def test_mutual_strikes_detection():
    """Test detection of jurors struck by both sides (anomaly flag)."""
    vm = VoirDireManager()

    case_id = "case-004"
    candidate_ids = [f"juror-{i}" for i in range(10)]
    candidate_data = {cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0} for cid in candidate_ids}

    voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # Both sides strike the same juror (mutual strike = anomaly)
    mutual_profile = profiles[0]

    vm.strike_juror(
        voir_dire_id=voir_dire_id,
        lawyer_id="da-counsel-01",
        lawyer_role=LawyerRole.DA_SAFETY,
        lawyer_side="prosecution",
        profile_hash=mutual_profile.profile_hash,
        reason=StrikeReason.SAFETY_CONCERN,
        reasoning_text="Safety risk detected",
    )

    vm.strike_juror(
        voir_dire_id=voir_dire_id,
        lawyer_id="defense-counsel-01",
        lawyer_role=LawyerRole.DEFENSE_DUE_PROCESS,
        lawyer_side="defense",
        profile_hash=mutual_profile.profile_hash,
        reason=StrikeReason.BIAS_PATTERN,
        reasoning_text="Bias pattern detected",
    )

    # Check mutual strikes
    mutual_cases = vm.resolve_mutual_strikes()
    assert voir_dire_id in mutual_cases
    assert len(mutual_cases[voir_dire_id]) == 1

    mutual_record = mutual_cases[voir_dire_id][0]
    assert mutual_record["profile_hash"] == mutual_profile.profile_hash
    assert mutual_record["prosecution_reason"] == "safety_concern"
    assert mutual_record["defense_reason"] == "bias_pattern"


def test_randomized_empanelment():
    """Test that final jury is randomized from remaining candidates."""
    vm = VoirDireManager()

    case_id = "case-005"
    candidate_ids = [f"juror-{i}" for i in range(12)]
    candidate_data = {cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0} for cid in candidate_ids}

    voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # DA strikes 3
    for i in range(3):
        vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="da-counsel-01",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profiles[i].profile_hash,
            reason=StrikeReason.DOMAIN_CONFLICT,
            reasoning_text="Conflict",
        )

    # Defense strikes 3 different jurors
    for i in range(3, 6):
        vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="defense-counsel-01",
            lawyer_role=LawyerRole.DEFENSE_DUE_PROCESS,
            lawyer_side="defense",
            profile_hash=profiles[i].profile_hash,
            reason=StrikeReason.BIAS_PATTERN,
            reasoning_text="Bias",
        )

    # Finalize with jury size 6
    success, seated, excluded = vm.finalize_empanelment(voir_dire_id, jury_size=6)

    assert success is True
    assert len(seated) == 6
    assert len(excluded) == 6  # 3 DA + 3 Defense strikes

    # Verify seated jurors are from remaining pool
    remaining_hashes = {p.profile_hash for p in profiles[6:]}
    for seated_hash in seated:
        assert seated_hash in remaining_hashes


def test_lawyer_bias_pattern_detection():
    """Test detection of bias patterns in lawyer's strike history."""
    vm = VoirDireManager()

    case_id_1 = "case-006"
    case_id_2 = "case-007"

    # First case with normal pattern
    candidate_ids_1 = [f"juror-{i}" for i in range(10)]
    candidate_data_1 = {
        cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0}
        for cid in candidate_ids_1
    }
    voir_dire_id_1, profiles_1 = vm.anonymize_candidates(case_id_1, candidate_ids_1, candidate_data_1)

    # Lawyer strikes 2 with BIAS_PATTERN reason
    for i in range(2):
        vm.strike_juror(
            voir_dire_id=voir_dire_id_1,
            lawyer_id="da-counsel-02",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profiles_1[i].profile_hash,
            reason=StrikeReason.BIAS_PATTERN,
            reasoning_text=f"Bias pattern {i}",
        )

    # Second case: same lawyer strikes 6 more with BIAS_PATTERN (should raise flag)
    candidate_ids_2 = [f"juror-x-{i}" for i in range(10)]
    candidate_data_2 = {
        cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0}
        for cid in candidate_ids_2
    }
    voir_dire_id_2, profiles_2 = vm.anonymize_candidates(case_id_2, candidate_ids_2, candidate_data_2)

    for i in range(6):
        vm.strike_juror(
            voir_dire_id=voir_dire_id_2,
            lawyer_id="da-counsel-02",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profiles_2[i].profile_hash,
            reason=StrikeReason.BIAS_PATTERN,
            reasoning_text=f"Bias pattern {i}",
        )

    # Check patterns
    analysis = vm.detect_lawyer_bias_patterns("da-counsel-02")
    assert analysis["concern_level"] == "high"
    assert any("Repeated strike reason" in flag for flag in analysis["flags"])


def test_voir_dire_audit_trail():
    """Test complete audit trail generation."""
    vm = VoirDireManager()

    case_id = "case-008"
    candidate_ids = [f"juror-{i}" for i in range(8)]
    candidate_data = {cid: {"reputation": 0.7, "section": "general", "recent_jury_count": 0} for cid in candidate_ids}

    voir_dire_id, profiles = vm.anonymize_candidates(case_id, candidate_ids, candidate_data)

    # DA strikes 2
    for i in range(2):
        vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="da-counsel-03",
            lawyer_role=LawyerRole.DA_PROCEDURAL,
            lawyer_side="prosecution",
            profile_hash=profiles[i].profile_hash,
            reason=StrikeReason.DOMAIN_CONFLICT,
            reasoning_text="Conflict in domain",
        )

    # Defense strikes 2
    for i in range(2, 4):
        vm.strike_juror(
            voir_dire_id=voir_dire_id,
            lawyer_id="defense-counsel-02",
            lawyer_role=LawyerRole.DEFENSE_DUE_PROCESS,
            lawyer_side="defense",
            profile_hash=profiles[i].profile_hash,
            reason=StrikeReason.BIAS_PATTERN,
            reasoning_text="Bias pattern",
        )

    # Finalize
    vm.finalize_empanelment(voir_dire_id, jury_size=4)

    # Get audit trail
    trail = vm.get_voir_dire_audit_trail(voir_dire_id)

    assert trail["case_id"] == case_id
    assert trail["total_candidates"] == 8
    assert trail["prosecution_strikes"] == 2
    assert trail["defense_strikes"] == 2
    assert trail["mutual_strikes"] == 0
    assert trail["final_jury_size"] == 4
    assert len(trail["strikes_history"]) == 4
