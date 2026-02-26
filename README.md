# salt-miner

A parallel Rust tool for mining a zkSync `CREATE2` salt that produces a deterministic vanity address on zkSync-based chains (e.g., **Abstract**).

## What This Tool Does

This tool brute-forces a 32-byte `salt` value such that the zkSync `CREATE2` address derivation formula produces a specific **target address**. It is preconfigured to find the salt for deploying the **RemoteHopV2 Proxy** (`FraxUpgradeableProxy`) to `0x0000006D38568b00B457580b734e0076C62de659` on Abstract.

## Why zkSync Address Derivation Differs from Standard EVM

On standard EVM chains, `CREATE2` derives addresses with:

```
address = keccak256(0xff ++ deployer ++ salt ++ keccak256(initcode))[12:]
```

On zkSync-based chains (like Abstract), the formula is:

```
address = keccak256(
    keccak256("zksyncCreate2")     // constant prefix
    ++ bytes32(deployer)           // 32-byte padded deployer address
    ++ salt                        // 32-byte salt (what we mine)
    ++ bytecodeHash                // zkSync-specific bytecode hash from zksolc
    ++ keccak256(constructorArgs)  // hash of ABI-encoded constructor arguments
)[12:]
```

The `bytecodeHash` is not `keccak256(initcode)` — it is a special hash produced by `zksolc` (the zkSync Solidity compiler) that encodes the contract's bytecode in a different format.

## Deployment Flow

```
EOA (0x54f9b12...17bc)
  │
  └─► Nick's Factory (0x4e59b44...956C)  ← CREATE2 deployer (appears in address formula)
          │
          └─► CREATE2 → FraxUpgradeableProxy at target address
                          constructor(
                            logic  = RemoteHopV2 (0x0000000087ED...3C6),
                            admin  = EOA / msg.sender (0x54f9b12...7bc),
                            data   = ""
                          )
```

**Key distinction:** Nick's Factory is the `CREATE2` sender used in the address formula, but the EOA (`msg.sender` in the Forge script context) is passed as the `admin` constructor argument — _not_ Nick's Factory.

Reference Arbitrum deployment: <https://arbiscan.io/tx/0x836f1218dcacf6b539e1f8edfad7963aa284cc23c92fe78330cbf5a9f1f905cb>

## Configuration Values

| Parameter | Value | Description |
|---|---|---|
| Target address | `0x0000006D38568b00B457580b734e0076C62de659` | Desired proxy deployment address |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` | Nick's Factory |
| Bytecode hash | `0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a` | zkSync bytecode hash of `FraxUpgradeableProxy` |
| Implementation | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` | RemoteHopV2 |
| Admin (EOA) | `0x54f9b12743a7deec0ea48721683cbebedc6e17bc` | Proxy admin / msg.sender |

All values are hardcoded as CLI defaults so the tool works out of the box.

## How to Get the zkSync Bytecode Hash

The `bytecodeHash` is produced when compiling with `zksolc` or `foundry-zksync`:

```bash
# Using foundry-zksync
forge build --zksync

# The hash appears in the compiled artifact under:
# out/FraxUpgradeableProxy.sol/FraxUpgradeableProxy.json → bytecode.object (first 32 bytes encode the hash)
# Or check zksolc output for the "bytecodeHash" field.
```

## Running Locally

### Prerequisites

- [Rust](https://rustup.rs/) (stable)

### Build and run with defaults

```bash
cargo run --release
```

### Run with custom arguments

```bash
cargo run --release -- \
  --target          0x0000006D38568b00B457580b734e0076C62de659 \
  --deployer        0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash   0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a \
  --implementation  0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6 \
  --admin           0x54f9b12743a7deec0ea48721683cbebedc6e17bc
```

### Expected output

```
Mining zkSync CREATE2 salt...
  Target address : 0x0000006d38568b00b457580b734e0076c62de659
  Deployer       : 0x4e59b44847b379578588920ca78fbf26c0b4956c
  Bytecode hash  : 0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a
  Implementation : 0x0000000087ed0dd8b999ae6c7c30f95e9707a3c6
  Admin (EOA)    : 0x54f9b12743a7deec0ea48721683cbebedc6e17bc
  Constructor args hash: 0x...

Found salt after N iterations in X.XXs!
  Salt (bytes32) : 0x000000000000000000000000000000000000000000000000000000000000XXXX
  Verified: derived address matches target 0x0000006d38568b00b457580b734e0076c62de659
```

The target has **5 leading zero nibbles**, so the salt is typically found within seconds.

## Running via GitHub Actions

1. Go to **Actions** → **Mine zkSync CREATE2 Salt** in this repository.
2. Click **Run workflow**.
3. Optionally override any of the default input values.
4. The workflow will build the binary and run the miner (timeout: 6 hours).

## Using the Mined Salt in a Forge Deploy Script

Once you have the `bytes32` salt, plug it directly into your Forge deployment:

```solidity
bytes32 salt = 0x000000000000000000000000000000000000000000000000000000000000XXXX;

FraxUpgradeableProxy proxy = new FraxUpgradeableProxy{salt: salt}(
    address(implementation),  // RemoteHopV2
    msg.sender,               // admin (EOA)
    ""                        // empty data
);
```

Deploy via Nick's Factory so the `CREATE2` sender matches the value used during mining:

```bash
cast send 0x4e59b44847b379578588920cA78FbF26c0B4956C \
  "$(cat deploy_calldata.hex)" \
  --rpc-url <ABSTRACT_RPC>
```
