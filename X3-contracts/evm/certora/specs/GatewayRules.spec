/* X3ExternalGateway Certora Verification Spec */

/* Rule 1: Locked assets cannot go negative */
invariant totalLockedNonNegative(address token)
    totalLocked(token) >= 0

/* Rule 2: Double-release protection — used messages cannot be reused */
rule noDoubleRelease(bytes32 messageId, address token, address recipient, uint256 amount, bytes sender, bytes proof) {
    require usedMessages(messageId) == false;
    releaseFromX3(messageId, token, recipient, amount, sender, proof);
    assert usedMessages(messageId) == true;
}

/* Rule 3: Release cannot exceed locked balance */
rule releaseCannotExceedLocked(bytes32 messageId, address token, address recipient, uint256 amount, bytes sender, bytes proof) {
    uint256 lockedBefore = totalLocked(token);
    require amount > lockedBefore;
    releaseFromX3(messageId, token, recipient, amount, sender, proof);
    assert false, "Release should revert when amount > locked";
}

/* Rule 4: Paused gateway blocks deposits */
rule pausedBlocksDeposits(address token, bytes x3Recipient, uint256 amount, uint256 nonce) {
    require paused() == true;
    depositToX3(token, x3Recipient, amount, nonce);
    assert false, "Deposit should revert when paused";
}

/* Rule 5: Paused gateway blocks releases */
rule pausedBlocksReleases(bytes32 messageId, address token, address recipient, uint256 amount, bytes sender, bytes proof) {
    require paused() == true;
    releaseFromX3(messageId, token, recipient, amount, sender, proof);
    assert false, "Release should revert when paused";
}

/* Rule 6: Unsupported token deposits revert */
rule unsupportedTokenReverts(address token, bytes x3Recipient, uint256 amount, uint256 nonce) {
    require supportedTokens(token) == false;
    depositToX3(token, x3Recipient, amount, nonce);
    assert false, "Deposit of unsupported token should revert";
}
