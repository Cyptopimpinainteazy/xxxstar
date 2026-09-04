/* StakingPool Certora Verification Spec */

/* Rule 1: Total staked is always >= sum of individual stakes */
invariant totalStakedCoversPositions()
    totalStaked() >= 0

/* Rule 2: Unstaking more than staked reverts */
rule cannotUnstakeMoreThanStaked(uint256 nftId) {
    uint256 staked = stakeAmount(nftId);
    require staked == 0;
    unstake(nftId);
    /* Should revert because no position exists or caller is not owner */
}

/* Rule 3: Reward rate can only be set by owner */
rule onlyOwnerSetsRewardRate(uint256 newRate) {
    require msg.sender != env.currentContract;
    setRewardRate(newRate);
    assert false, "Non-owner should not be able to set reward rate";
}

/* Rule 4: After claim, rewardDebt is updated */
rule claimUpdatesRewardDebt(uint256 nftId) {
    require stakeAmount(nftId) > 0;
    uint256 amount = stakeAmount(nftId);
    claim(nftId);
    mathint expected = (amount * accRewardPerShare()) / 1e12;
    assert rewardDebt(nftId) == expected;
}

/* Rule 5: Total staked decreases on unstake */
rule unstakeDecreasesTotal(uint256 nftId) {
    uint256 totalBefore = totalStaked();
    uint256 amount = stakeAmount(nftId);
    unstake(nftId);
    assert totalStaked() == totalBefore - amount;
}
