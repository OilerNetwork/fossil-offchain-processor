//! Ethereum Data Dispatcher Library
//!
//! This library provides tools for connecting to the Ethereum network and sending block hashes to a specified Starknet smart contract.
//!
//! # Overview
//!
//! The `eth-dispatcher` library is designed to:
//! - Establish a connection to the Ethereum blockchain.
//! - Relay block hashes to Starknet.
//! - Ensure that transactions are signed and sent with the correct gas estimation.
//! - Monitor transaction status and confirm successful submissions.
//!
//! # Modules
//!
//! - `relayer`: Contains the core functionalities for relaying messages between Ethereum and Starknet, including transaction management and message sending.

/// The `relayer` module handles the core functionalities for relaying messages between Ethereum and Starknet.
pub mod relayer;
