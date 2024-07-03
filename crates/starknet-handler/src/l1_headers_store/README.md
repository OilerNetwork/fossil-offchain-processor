# L1 Headers Store

The `L1HeadersStore` struct is responsible for interacting with a smart contract that stores L1 state roots on the StarkNet blockchain.

## Struct

### [`L1HeadersStore`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/l1_headers_store/l1_headers_store.rs#L16-L21)

```rust
pub struct L1HeadersStore {
    provider: JsonRpcClient<HttpTransport>,
    signer: LocalWallet,
    l1_headers_store: FieldElement,
    owner_account: FieldElement,
}
```

## Method

### [`new`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/l1_headers_store/l1_headers_store.rs#L24)

Creates a new instance of `L1HeadersStore`.

#### Arguments

- `rpc` - A string slice that holds the URL of the JSON-RPC endpoint.
- `l1_headers_store` - The field element representing the L1 headers store contract address.
- `signer` - The local wallet used for signing transactions.
- `owner_account` - The field element representing the owner's account address.

#### Returns

A new instance of `L1HeadersStore`.

---

### [`store_state_root`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/l1_headers_store/l1_headers_store.rs#L41)

Sends a transaction to the L1 headers store contract to store a state root.

#### Arguments

- `block_number` - The block number associated with the state root.
- `state_root` - The state root as a string.

#### Returns

A result containing the invocation transaction result or a handler error.

---

### [`get_state_root`](https://github.com/OilerNetwork/fossil-offchain-processor/blob/main/crates/starknet-handler/src/l1_headers_store/l1_headers_store.rs#L61)

Calls the L1 headers store contract to get the state root for a specific block number.

#### Arguments

- `block_number` - The block number for which the state root is requested.

#### Returns

A result containing a vector of field elements representing the state root or a handler error.

