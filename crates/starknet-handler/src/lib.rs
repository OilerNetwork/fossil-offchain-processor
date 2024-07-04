//! # StarkNet Handler
//!
//! `starknet_handler` is a Rust library designed to interact with StarkNet blockchain smart contracts.
//! It provides modules for handling errors, managing a fact registry, and storing L1 headers.
//!
//! ## Modules
//!
//! - `error`: Contains error types used throughout the library.
//! - `fact_registry`: Manages the fact registry smart contract interactions.
//! - `l1_headers_store`: Handles storage of L1 state roots on the StarkNet blockchain.
//! - `util`: Provides utility functions for internal use.
//!
//! ## Usage
//!
//! To use this library, add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! starknet_handler = "0.1.0"
//! ```
//!
//! ## Example
//!
//! Here is an example of how to create an `L1HeadersStore` instance and call `get_state_root`:
//!
//! ```rust,no_run
//! use dotenv::dotenv;
//! use std::str::FromStr;
//! use starknet::core::types::FieldElement;
//! use starknet::signers::{LocalWallet, SigningKey};
//! use starknet_handler::l1_headers_store::L1HeadersStore;
//!
//! #[tokio::main]
//! async fn main() {
//!     dotenv().ok();
//!     let private_key = dotenv::var("PRIVATE_KEY").unwrap();
//!
//!     let owner_account = dotenv::var("OWNER_ACCOUNT").unwrap();
//!     let owner_account = FieldElement::from_str(owner_account.as_str()).unwrap();
//!
//!     let signer = LocalWallet::from(SigningKey::from_secret_scalar(
//!         FieldElement::from_hex_be(&private_key).unwrap(),
//!     ));
//!
//!     let l1_headers_store =
//!         FieldElement::from_hex_be(dotenv::var("L1_HEADERS_STORE_ADDRESS").unwrap().as_str())
//!             .unwrap();
//!
//!     // NOTE: change block number once its stored
//!     let block_number = 20;
//!
//!     let contract = L1HeadersStore::new(
//!         "http://localhost:5050",
//!         l1_headers_store,
//!         signer,
//!         owner_account,
//!     );
//!     let res = contract.get_state_root(block_number).await;
//!
//!     println!("{:?}", res);
//! }
//! ```

pub mod error;
pub mod fact_registry;
pub mod l1_headers_store;
mod util;
