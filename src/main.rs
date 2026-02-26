use clap::Parser;
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tiny_keccak::{Hasher, Keccak};

/// Compute keccak256 of the given bytes.
fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256();
    k.update(data);
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    out
}

/// Derive the zkSync CREATE2 address.
///
/// Formula:
///   hash = keccak256(
///       keccak256("zksyncCreate2")   // 32 bytes
///       ++ deployer_padded           // 32 bytes (left-padded address)
///       ++ salt                      // 32 bytes
///       ++ bytecode_hash             // 32 bytes
///       ++ keccak256(constructor_args) // 32 bytes
///   )
///   address = hash[12..32]
fn derive_address(
    prefix: &[u8; 32],
    deployer: &[u8; 32],
    salt: &[u8; 32],
    bytecode_hash: &[u8; 32],
    constructor_args_hash: &[u8; 32],
) -> [u8; 20] {
    let mut input = [0u8; 160];
    input[0..32].copy_from_slice(prefix);
    input[32..64].copy_from_slice(deployer);
    input[64..96].copy_from_slice(salt);
    input[96..128].copy_from_slice(bytecode_hash);
    input[128..160].copy_from_slice(constructor_args_hash);

    let hash = keccak256(&input);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    addr
}

/// Parse a hex string (with or without 0x prefix) into a fixed-size byte array.
fn parse_hex<const N: usize>(s: &str) -> Result<[u8; N], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("hex decode error: {e}"))?;
    if bytes.len() != N {
        return Err(format!("expected {} bytes, got {}", N, bytes.len()));
    }
    let mut arr = [0u8; N];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Left-pad a 20-byte address to 32 bytes.
fn address_to_bytes32(addr: &[u8; 20]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(addr);
    out
}

/// Build the ABI-encoded constructor args for FraxUpgradeableProxy:
///   abi.encode(address logic, address admin, bytes memory data)
/// with empty `data`.
///
/// Layout (4 words = 128 bytes):
///   [0..32]   logic  (left-padded address)
///   [32..64]  admin  (left-padded address)
///   [64..96]  offset to `data` = 0x60 (96)
///   [96..128] length of `data` = 0
fn build_constructor_args(implementation: &[u8; 20], admin: &[u8; 20]) -> Vec<u8> {
    let mut args = vec![0u8; 128];
    args[12..32].copy_from_slice(implementation);
    args[44..64].copy_from_slice(admin);
    // offset to bytes data field
    args[95] = 0x60;
    // length of bytes data = 0 (already zeroed)
    args
}

#[derive(Parser)]
#[command(name = "mine-zksync-salt", about = "Mine a zkSync CREATE2 salt for a target address")]
struct Args {
    /// Target proxy address (20-byte hex, 0x-prefixed)
    #[arg(long, default_value = "0x0000006D38568b00B457580b734e0076C62de659")]
    target: String,

    /// CREATE2 deployer address — Nick's Factory (20-byte hex, 0x-prefixed)
    #[arg(long, default_value = "0x4e59b44847b379578588920cA78FbF26c0B4956C")]
    deployer: String,

    /// zkSync bytecode hash of FraxUpgradeableProxy (32-byte hex, 0x-prefixed)
    #[arg(
        long,
        default_value = "0x010000cfc5ec4899fe1afb5ee91e52433aa1089de03eb4fbe3dbb67dc1a6f55a"
    )]
    bytecode_hash: String,

    /// Implementation address — RemoteHopV2 (20-byte hex, 0x-prefixed)
    #[arg(long, default_value = "0x0000000087ED0dD8b999aE6C7c30f95e9707a3C6")]
    implementation: String,

    /// Proxy admin — EOA / msg.sender (20-byte hex, 0x-prefixed)
    #[arg(long, default_value = "0x54f9b12743a7deec0ea48721683cbebedc6e17bc")]
    admin: String,
}

fn main() {
    let args = Args::parse();

    let target: [u8; 20] = parse_hex(&args.target).expect("invalid --target");
    let deployer_addr: [u8; 20] = parse_hex(&args.deployer).expect("invalid --deployer");
    let bytecode_hash: [u8; 32] = parse_hex(&args.bytecode_hash).expect("invalid --bytecode-hash");
    let implementation: [u8; 20] = parse_hex(&args.implementation).expect("invalid --implementation");
    let admin: [u8; 20] = parse_hex(&args.admin).expect("invalid --admin");

    // Pre-compute constant values
    let prefix = keccak256(b"zksyncCreate2");
    let deployer_padded = address_to_bytes32(&deployer_addr);
    let constructor_args = build_constructor_args(&implementation, &admin);
    let constructor_args_hash = keccak256(&constructor_args);

    println!("Mining zkSync CREATE2 salt...");
    println!("  Target address : 0x{}", hex::encode(target));
    println!("  Deployer       : 0x{}", hex::encode(deployer_addr));
    println!("  Bytecode hash  : 0x{}", hex::encode(bytecode_hash));
    println!("  Implementation : 0x{}", hex::encode(implementation));
    println!("  Admin (EOA)    : 0x{}", hex::encode(admin));
    println!("  Constructor args hash: 0x{}", hex::encode(constructor_args_hash));

    let start = Instant::now();
    let total_checked = Arc::new(AtomicU64::new(0));
    let chunk_size: u64 = 1_000_000;
    let log_interval: u64 = 100_000_000;

    let result = (0u64..)
        .map(|chunk_start| chunk_start * chunk_size)
        .take_while(|&start_val| start_val < u64::MAX - chunk_size)
        .find_map(|chunk_start| {
            let found = (chunk_start..chunk_start + chunk_size)
                .into_par_iter()
                .find_map_any(|i| {
                    let mut salt = [0u8; 32];
                    salt[24..].copy_from_slice(&i.to_be_bytes());

                    let addr = derive_address(
                        &prefix,
                        &deployer_padded,
                        &salt,
                        &bytecode_hash,
                        &constructor_args_hash,
                    );
                    if addr == target {
                        Some((i, salt))
                    } else {
                        None
                    }
                });

            let checked = total_checked.fetch_add(chunk_size, Ordering::Relaxed) + chunk_size;
            if checked % log_interval == 0 || found.is_some() {
                let elapsed = start.elapsed().as_secs_f64();
                let rate = checked as f64 / elapsed / 1_000_000.0;
                println!(
                    "  Checked {:>6}M iterations ({:.2} M/s)...",
                    checked / 1_000_000,
                    rate
                );
            }

            found
        });

    match result {
        Some((i, salt)) => {
            let elapsed = start.elapsed();
            println!("\nFound salt after {} iterations in {:.2?}!", i + 1, elapsed);
            println!("  Salt (bytes32) : 0x{}", hex::encode(salt));

            // Verify
            let addr = derive_address(
                &prefix,
                &deployer_padded,
                &salt,
                &bytecode_hash,
                &constructor_args_hash,
            );
            assert_eq!(
                addr, target,
                "BUG: derived address does not match target!"
            );
            println!("  Verified: derived address matches target 0x{}", hex::encode(addr));
        }
        None => {
            eprintln!("No salt found in search range.");
            std::process::exit(1);
        }
    }
}
