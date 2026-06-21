methods {
    function totalStaked() external returns (uint256)
    function stakeAmount(uint256) external returns (uint256)
    function rewardDebt(uint256) external returns (uint256)
    function accRewardPerShare() external returns (uint256)
    function stake(uint256) external
    function unstake(uint256) external
    function claim(uint256) external
    function setRewardRate(uint256) external
    function updatePool() external
}
