methods {
    function devSplitBps() external returns (uint256)
    function daoSplitBps() external returns (uint256)
    function lpSplitBps() external returns (uint256)
    function setSplits(uint256,uint256,uint256) external
    function routeFee(address,uint256,string) external
    function accounting(address) external returns (uint256)
}
