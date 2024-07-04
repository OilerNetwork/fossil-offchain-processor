//! Module for generating type-safe bindings to the L1MessageSender contract.
//!
//! This module uses `abigen` macro to generate Rust bindings for the L1MessageSender contract,
//! allowing for type-safe interactions with the contract.

use ethers::middleware::contract::abigen;

/// Generates Rust bindings for the L1MessageSender contract.
///
/// The contract includes the following functions:
///
/// - `constructor(address snMessaging, uint256 l2RecipientAddr)`
/// - `function l2RecipientAddr() public view returns (uint256)`
/// - `function sendExactParentHashToL2(uint256 blockNumber_) external payable`
/// - `function sendLatestParentHashToL2() external payable`
///
/// These bindings enable type-safe interactions with the contract from Rust code.

abigen!(
    L1MessageSender,
    r#"[
        constructor(address snMessaging, uint256 l2RecipientAddr)
        function l2RecipientAddr() public view returns (uint256)
        function sendExactParentHashToL2(uint256 blockNumber_) external payable
        function sendLatestParentHashToL2() external payable
    ]"#;
);
