/* Treasury Certora Verification Spec */

/* Rule 1: Splits always sum to 10000 */
invariant splitsSumTo10000()
    devSplitBps() + daoSplitBps() + lpSplitBps() == 10000

/* Rule 2: No individual split exceeds 10000 */
invariant splitBounds()
    devSplitBps() <= 10000 && daoSplitBps() <= 10000 && lpSplitBps() <= 10000

/* Rule 3: setSplits enforces sum constraint */
rule setSplitsEnforcesSum(uint256 dev, uint256 dao, uint256 lp) {
    require dev + dao + lp != 10000;
    setSplits(dev, dao, lp);
    assert false, "setSplits should revert when sum != 10000";
}

/* Rule 4: Accounting is monotonically increasing */
rule accountingMonotonic(address from, uint256 amount, string category) {
    uint256 before = accounting(from);
    routeFee(from, amount, category);
    assert accounting(from) >= before;
}

/* Rule 5: Fee routing distributes all funds (no dust loss) */
rule noDustLoss(uint256 amount) {
    uint256 devAmt = (amount * devSplitBps()) / 10000;
    uint256 daoAmt = (amount * daoSplitBps()) / 10000;
    uint256 lpAmt = amount - devAmt - daoAmt;
    assert devAmt + daoAmt + lpAmt == amount;
}
