# FactRegistry

The `FactRegistry` struct is responsible for interacting with a smart contract that serves as a registry for various proofs (e.g., storage proofs, account proofs). It communicates with a StarkNet blockchain via JSON-RPC.

## Struct

### [`FactRegistry`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L21-L26)

```rust
pub struct FactRegistry {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    fact_registry: FieldElement,
    owner_account: FieldElement,
}
```

## Method

### [`new`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L30)

Creates a new instance of `FactRegistry`.

#### Arguments

- `rpc` - A string slice that holds the URL of the JSON-RPC endpoint.
- `fact_registry` - The field element representing the fact registry contract address.
- `signer` - The local wallet used for signing transactions.
- `owner_account` - The field element representing the owner's account address.

#### Returns

A new instance of `FactRegistry`.

---

### [`prove_storage`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L47)

Sends a transaction to the fact registry contract to prove storage.


#### Arguments

- `block_number` - The block number at which the proof is valid.
- `account_address` - The U256 representation of the account address.
- `storage_proof` - The storage proof data.
- `slot` - The storage slot as a string.

#### Returns

A result containing the invocation transaction result or a handler error.

---

### [`prove_account`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L79)

Sends a transaction to the fact registry contract to prove an account.


#### Arguments

- `block_number` - The block number at which the proof is valid.
- `account_proof` - The account proof data.

#### Returns

A result containing the invocation transaction result or a handler error.

---

### [`get_storage`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L104)

Calls the fact registry contract to get storage data.


#### Arguments

- `block_number` - The block number at which the data is valid.
- `account_address` - The U256 representation of the account address.
- `slot` - The storage slot as a string.

#### Returns

A result containing a vector of field elements representing the storage data or a handler error.

---

### [`get_verified_account_hash`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/fact_registry/fact_registry.rs#L125)

Calls the fact registry contract to get the verified account hash.


#### Arguments

- `block_number` - The block number at which the data is valid.
- `account_address` - The U256 representation of the account address.

#### Returns

A result containing a vector of field elements representing the account hash or a handler error.


