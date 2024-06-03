use ethers::middleware::contract::abigen;

abigen!(
    L1MessageSender,
    r#"[
        constructor(address snMessaging, uint256 l2RecipientAddr)
        function l2RecipientAddr() public view returns (uint256)
        function sendExactParentHashToL2(uint256 blockNumber_) external payable
        function sendLatestParentHashToL2() external payable
    ]"#;
);
