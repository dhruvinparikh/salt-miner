# salt-miner

A parallel Rust tool for mining a zkSync `CREATE2` salt that produces a deterministic vanity address on zkSync-based chains (e.g., **Abstract**).

## What This Tool Does

This tool brute-forces a 32-byte `salt` value such that the zkSync `CREATE2` address derivation formula produces a specific **target address**. It supports three contracts:

- **`impl`** — RemoteHopV2 implementation (no constructor args)
- **`proxy`** — FraxUpgradeableProxy (constructor args: implementation + admin)
- **`remote-admin`** — RemoteAdmin (constructor args: frxUsdOft + remoteHop + msig)

All three contracts use **Nick's Factory** (`0x4e59b44847b379578588920cA78FbF26c0B4956C`) as the CREATE2 deployer.

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

## Configuration Values

### RemoteHopV2 Implementation (`impl`)

| Parameter | Value | Description |
|---|---|---|
| Target address | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` | Desired implementation address |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` | Nick's Factory |
| Bytecode hash | `0x0100075b76ae9ac5481afa04f066daeb43f25b709358040665df9acce858942a` | zkSync bytecode hash of `RemoteHopV2` |
| Constructor args | _(none)_ | `keccak256("")` |

### FraxUpgradeableProxy (`proxy`)

| Parameter | Value | Description |
|---|---|---|
| Target address | `0x0000006D38568b00B457580b734e0076C62de659` | Desired proxy deployment address |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` | Nick's Factory |
| Bytecode hash | `0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a` | zkSync bytecode hash of `FraxUpgradeableProxy` |
| Implementation | `0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6` | RemoteHopV2 |
| Admin (EOA) | `0x54f9b12743a7deec0ea48721683cbebedc6e17bc` | Proxy admin / msg.sender |
| Constructor args | `abi.encode(address logic, address admin, bytes(""))` | 128 bytes |

### RemoteAdmin (`remote-admin`)

| Parameter | Value | Description |
|---|---|---|
| Target address | `0x954286118E93df807aB6f99aE0454f8710f0a8B9` | Desired RemoteAdmin address |
| CREATE2 deployer | `0x4e59b44847b379578588920cA78FbF26c0B4956C` | Nick's Factory |
| Bytecode hash | `0x0100008bc5b4435f4bf1420fec25c30c5d5a001616032a936e255af46b1a2fd8` | zkSync bytecode hash of `RemoteAdmin` |
| frxUSD OFT | `0xEa77c590Bb36c43ef7139cE649cFBCFD6163170d` | frxUSD OFT address |
| RemoteHop | `0x0000006D38568b00B457580b734e0076C62de659` | RemoteHop proxy address |
| Msig | `0x5f25218ed9474b721d6a38c115107428E832fA2E` | Multisig address |
| Constructor args | `abi.encode(address frxUsdOft, address remoteHop, address msig)` | 96 bytes |

## How to Get the zkSync Bytecode Hash

The `bytecodeHash` is produced when compiling with `zksolc` or `foundry-zksync`:

```bash
# Using foundry-zksync
forge build --zksync

# The hash appears in the compiled artifact under:
# out/<Contract>.sol/<Contract>.json → bytecode.object (first 32 bytes encode the hash)
# Or check zksolc output for the "bytecodeHash" field.
```

## Running Locally

### Prerequisites

- [Rust](https://rustup.rs/) (stable)

### Mine RemoteHopV2 implementation salt

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
# impl subcommand
cargo run --release -- impl \
  --target        0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6 \
  --deployer      0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash 0x0100075b76ae9ac5481afa04f066daeb43f25b709358040665df9acce858942a

# proxy subcommand
cargo run --release -- proxy \
  --target          0x0000006D38568b00B457580b734e0076C62de659 \
  --deployer        0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash   0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a \
  --implementation  0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6 \
  --admin           0x54f9b12743a7deec0ea48721683cbebedc6e17bc

# remote-admin subcommand
cargo run --release -- remote-admin \
  --target        0x954286118E93df807aB6f99aE0454f8710f0a8B9 \
  --deployer      0x4e59b44847b379578588920cA78FbF26c0B4956C \
  --bytecode-hash 0x0100008bc5b4435f4bf1420fec25c30c5d5a001616032a936e255af46b1a2fd8 \
  --frxusd-oft    0xEa77c590Bb36c43ef7139cE649cFBCFD6163170d \
  --remote-hop    0x0000006D38568b00B457580b734e0076C62de659 \
  --msig          0x5f25218ed9474b721d6a38c115107428E832fA2E
```

## Running via GitHub Actions

Three separate workflows are available, each mining a different contract:

| Workflow | Contract | Action |
|---|---|---|
| **Mine RemoteHopV2 Implementation Salt** | RemoteHopV2 impl | `impl` subcommand |
| **Mine FraxUpgradeableProxy Salt** | FraxUpgradeableProxy | `proxy` subcommand |
| **Mine RemoteAdmin Salt** | RemoteAdmin | `remote-admin` subcommand |

1. Go to **Actions** in this repository.
2. Select the workflow for the contract you want to mine.
3. Click **Run workflow**.
4. Optionally override any of the default input values.
5. The workflow will build the binary and run the miner (timeout: 6 hours).

## Using the Mined Salt in a Forge Deploy Script

Once you have the `bytes32` salt, plug it directly into your Forge deployment.

### RemoteHopV2 implementation

```solidity
bytes32 salt = 0x...;
RemoteHopV2 impl = new RemoteHopV2{salt: salt}();
```

### FraxUpgradeableProxy

```solidity
bytes32 salt = 0x...;

FraxUpgradeableProxy proxy = new FraxUpgradeableProxy{salt: salt}(
    address(implementation),  // RemoteHopV2
    msg.sender,               // admin (EOA)
    ""                        // empty data
);
```

### RemoteAdmin

```solidity
bytes32 salt = 0x...;

RemoteAdmin remoteAdmin = new RemoteAdmin{salt: salt}(
    address(frxUsdOft),
    address(remoteHop),
    address(msig)
);
```

Deploy via Nick's Factory so the `CREATE2` sender matches the value used during mining:

```bash
cast send 0x4e59b44847b379578588920cA78FbF26c0B4956C \
  "$(cat deploy_calldata.hex)" \
  --rpc-url <ABSTRACT_RPC>
```
