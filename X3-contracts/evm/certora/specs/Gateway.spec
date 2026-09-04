methods {
    function totalLocked(address) external returns (uint256)
    function usedMessages(bytes32) external returns (bool)
    function supportedTokens(address) external returns (bool)
    function paused() external returns (bool)
    function depositToX3(address,bytes,uint256,uint256) external
    function releaseFromX3(bytes32,address,address,uint256,bytes,bytes) external
    function setSupportedToken(address,bool,uint256,uint256) external
    function setPaused(bool) external
}
