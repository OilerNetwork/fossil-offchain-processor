# Offchain Processor Components Requirements

## Running the Offchain Processor
```bash

cargo run --release --bin request-manager

```

## Example API Request

```bash

curl --location 'http://localhost:8000/get-storage' \
--header 'Content-Type: application/json' \
--data '{
	"account_address":"0xe7f1725e7734ce288f8367e1bb143e90bb3f0512",
    "storage_keys": ["0x0"],
    "block_number": 4,
    "slot": "0x0"
}'

```

### 1. Ethereum Data Dispatcher

**Goal:**

- Connect to the Ethereum network to send block hashes to a specified Starknet smart contract.

**Responsibilities:**

- Establish a connection to the Ethereum blockchain.
- Call the  function to relay the block hashes on Ethereum
- Ensure that transactions are signed and sent with the correct gas estimation.
- Monitor transaction status and confirm successful submissions.

### 2. Starknet Contract Handler

**Goal:**

- Interact with Starknet smart contracts to perform various read and write operations and verify proofs.

**Responsibilities:**

- Connect to the Starknet blockchain.
- Execute read operations on specified Starknet smart contracts.
- Execute write operations on specified Starknet smart contracts.
- Verify proofs received from the Ethereum Proof Generator and store them on Starknet.
- Ensure data integrity and confirmation of successful transactions on Starknet.

### 3. Ethereum Proof Generator

**Goal:**

- Generate storage proofs and retrieve block headers from Ethereum, encoding them into RLP format.

**Responsibilities:**

- Connect to Ethereum nodes to access blockchain data.
- Retrieve block headers for specified blocks.
- Produce storage proofs for given Ethereum accounts and blocks.
- Encode the retrieved block headers and storage proofs into RLP format.
- Return the encoded data to the User Request Manager for further processing.

### 4. User Request Manager

**Goal:**

- Manage user requests for specific historical Ethereum account storage values and coordinate interactions with backend components.

**Responsibilities:**

- Receive and parse user requests for Ethereum account storage values.
- Check if the requested storage value is already available.
- If not available, coordinate with the Ethereum Data Dispatcher to retrieve necessary block hashes.
- Store the retrieved block hashes on Starknet using the Starknet Contract Handler.
- Verify if the requested account has already been proved on Starknet.
- If the account is proved, produce a storage proof using the Ethereum Proof Generator and verify it on Starknet.
- If the account is not proved, generate the account proof first, verify it on Starknet, then produce and verify the storage proof.
- Communicate the final result (storage found or storage not found) back to the user.
- Ensure the storage value is available for use on Starknet once all verifications are complete.
