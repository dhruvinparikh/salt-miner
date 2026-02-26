# salt-miner

A parallel Rust tool for mining zkSync `CREATE2` salts that produce deterministic vanity addresses on zkSync-based chains (e.g., **Abstract**).

## What This Tool Does

This tool brute-forces a 32-byte `salt` value such that the zkSync `CREATE2` address derivation formula produces a specific **target address**. It supports mining salts for three contracts:

| Contract | Target Address | Subcommand |
|---|---|---|
| RemoteHopV2 (Implementation) | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` | `impl` |
| FraxUpgradeableProxy | `0x0000006D38568b00B457580b734e0076C62de659` | `proxy` |
| RemoteAdmin | `0x954286118E93df807aB6f99aE0454f8710f0a8B9` | `remote-admin` |

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
          └─► CREATE2 → Contract at target address
```

**Key distinction:** Nick's Factory is the `CREATE2` sender used in the address formula, but the EOA (`msg.sender` in the Forge script context) is passed as constructor arguments where needed — _not_ Nick's Factory.

Reference Arbitrum deployment: <https://arbiscan.io/tx/0x836f1218dcacf6b539e1f8edfad7963aa284cc23c92fe78330cbf5a9f1f905cb>

## Contract Configurations

### 1. RemoteHopV2 Implementation (`impl`)

| Parameter | Value |
|---|---|
| Target address | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` |
| Bytecode hash | `0x0100075b76ae9ac5481afa04f066daeb43f25b709358040665df9acce858942a` |
| Constructor args | **None** (empty bytes) |

### 2. FraxUpgradeableProxy (`proxy`)

| Parameter | Value |
|---|---|
| Target address | `0x0000006D38568b00B457580b734e0076C62de659` |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` |
| Bytecode hash | `0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a` |
| Implementation | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` |
| Admin (EOA) | `0x54f9b12743a7deec0ea48721683cbebedc6e17bc` |

Constructor: `(address logic, address admin, bytes memory data)` with empty data.

### 3. RemoteAdmin (`remote-admin`)

| Parameter | Value |
|---|---|
| Target address | `0x954286118E93df807aB6f99aE0454f8710f0a8B9` |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` |
| Bytecode hash | `0x0100008bc5b4435f4bf1420fec25c30c5d5a001616032a936e255af46b1a2fd8` |
| frxUSD OFT | `0xEa77c590Bb36c43ef7139cE649cFBCFD6163170d` |
| RemoteHop | `0x0000006D38568b00B457580b734e0076C62de659` |
| Multisig | `0x5f25218ed9474b721d6a38c115107428E832fA2E` |

Constructor: `(address frxUsdOft, address remoteHop, address msig)`.

## How to Get the zkSync Bytecode Hash

The `bytecodeHash` is produced when compiling with `zksolc` or `foundry-zksync`:

```bash
# Using foundry-zksync
forge build --zksync

# The hash appears in the compiled artifact under:
# out/Contract.sol/Contract.json → bytecode.object (first 32 bytes encode the hash)
# Or check zksolc output for the "bytecodeHash" field.
```

## Running Locally

### Prerequisites

- [Rust](https://rustup.rs/) (stable)

### Mine RemoteHopV2 Implementation salt

```bash
cargo run --release -- impl
```

### Mine FraxUpgradeableProxy salt

```bash
cargo run --release -- proxy
```

### Mine RemoteAdmin salt

```bash
cargo run --release -- remote-admin
```

### Run with custom arguments

```bash
# Implementation (no constructor args)
cargo run --release -- impl \
  --target          0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6 \
  --deployer        0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash   0x0100075b76ae9ac5481afa04f066daeb43f25b709358040665df9acce858942a

# Proxy
cargo run --release -- proxy \
  --target          0x0000006D38568b00B457580b734e0076C62de659 \
  --deployer        0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash   0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a \
  --implementation  0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6 \
  --admin           0x54f9b12743a7deec0ea48721683cbebedc6e17bc

# RemoteAdmin
cargo run --release -- remote-admin \
  --target          0x954286118E93df807aB6f99aE0454f8710f0a8B9 \
  --deployer        0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash   0x0100008bc5b4435f4bf1420fec25c30c5d5a001616032a936e255af46b1a2fd8 \
  --frxusd-oft      0xEa77c590Bb36c43ef7139cE649cFBCFD6163170d \
  --remote-hop      0x0000006D38568b00B457580b734e0076C62de659 \
  --msig            0x5f25218ed9474b721d6a38c115107428E832fA2E
```

## Running via GitHub Actions

Each contract has its own workflow that can be triggered independently:

1. **Mine RemoteHopV2 Implementation Salt** — Actions → "Mine RemoteHopV2 Implementation Salt" → Run workflow
2. **Mine FraxUpgradeableProxy Salt** — Actions → "Mine FraxUpgradeableProxy Salt" → Run workflow
3. **Mine RemoteAdmin Salt** — Actions → "Mine RemoteAdmin Salt" → Run workflow

All three can run in parallel. Each has a 6-hour timeout.

## Using the Mined Salt in a Forge Deploy Script

Once you have the `bytes32` salt, plug it directly into your Forge deployment:

```solidity
bytes32 salt = 0x000000000000000000000000000000000000000000000000000000000000XXXX;

// For proxy deployment
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