[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/RHGu4AQi)

# Rudi — Project Report

## 1. Introduction

**Rudi** is a non-custodial cryptocurrency wallet implemented in Rust. The core idea was to build a single wallet that manages keys and transactions across the three most widely used blockchain networks — Bitcoin, Ethereum, and Solana — from one unified interface. Rather than relying on a hosted service to hold keys, rudi generates a BIP-39 mnemonic phrase locally and derives all network-specific keys from it. The mnemonic is encrypted at rest using a user-supplied password and never leaves the machine in plaintext.

The project targets test networks (Bitcoin Testnet, Ethereum Sepolia, Solana Devnet) to allow safe experimentation without real funds. Its scope covers the essential operations any wallet must support: generating or importing a wallet, checking balances, sending tokens, and browsing transaction history. A session layer keeps the decrypted seed in memory for a bounded time so the user is not forced to re-enter their password on every command, while still zeroing that memory when the session expires or the process exits.

---

## 2. Requirements

**Wallet management**
- Generate a new 12-word BIP-39 mnemonic or import an existing one.
- Encrypt and persist the mnemonic locally (one file, `~/.crypto-wallet.dat`).
- Enforce strong passwords; support password changes without losing the mnemonic.

**Multi-network support**
- Derive correct addresses and private keys for Bitcoin, Ethereum, and Solana from the same seed.
- Query live balance from each network.
- Construct, sign, and broadcast transactions on all three networks.
- Fetch recent transaction history per network, with support for pagination via a `since_txid` cursor.

**Security**
- The seed must never be written to disk in plaintext.
- In-memory seed must be zeroed when the session expires (5-minute idle timeout, 1-hour hard cap) or the process exits.
- Salt and nonce must be randomly generated per save to prevent ciphertext reuse.

**Portability**
- Run on macOS, Linux, and Windows from the same codebase.

---

## 3. High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                         User / CLI                          │
└───────────────────────────┬─────────────────────────────────┘
                            │
              ┌─────────────▼─────────────┐
              │       Session Layer        │
              │  (in-memory seed, zeroize) │
              └──────┬──────────┬──────────┘
                     │          │
          ┌──────────▼──┐  ┌────▼──────────┐
          │  Key Derivation  │  Storage Layer│
          │  (tokens/)    │  (builders/)  │
          └──┬────┬────┬──┘  └────────────┘
             │    │    │
    ┌────────▼┐ ┌─▼────▼──┐ ┌───▼─────┐
    │  BTC    │ │   ETH   │ │   SOL   │
    │ Network │ │ Network │ │ Network │
    └────┬────┘ └────┬────┘ └────┬────┘
         │           │           │
    Blockstream  Alloy +     Raw JSON-RPC
    Testnet API  Sepolia RPC  (Devnet)
```

The architecture has four layers. The **storage layer** (`builders/storage.rs`) handles disk I/O — saving and loading the encrypted wallet file. The **session layer** (`session.rs`) holds the decrypted seed in memory and enforces time-based expiry with secure zeroing on drop. The **key derivation layer** (`tokens/`) translates the raw seed bytes into network-specific keypairs and addresses using each network's standard derivation path. The **network layer** (`networks/`) communicates with external APIs to check balances, submit transactions, and retrieve history.

---

## 4. Design Choices

### Encryption: AES-256-GCM + PBKDF2

The mnemonic at rest is encrypted with AES-256-GCM. The 256-bit key is derived from the user's password using PBKDF2-HMAC-SHA256 with 600 000 iterations, a random 16-byte salt, and a random 96-bit nonce. Every save generates fresh salt and nonce, so the same mnemonic encrypted with the same password produces a different ciphertext each time.

**Alternative considered:** scrypt or Argon2id, which are memory-hard and more resistant to GPU cracking. PBKDF2 was chosen for its simpler dependency footprint and wide auditability, and 600 000 iterations (the OWASP 2023 recommendation for SHA-256) keeps brute-force cost high enough for this scope.

### Key derivation: BIP-32/BIP-44 for Bitcoin and Ethereum, direct bytes for Solana

Bitcoin uses `m/44'/0'/0'/0/0` and Ethereum uses `m/44'/60'/0'/0/0`, both derived via the `bitcoin` crate's BIP-32 implementation. Ethereum's `PrivateKeySigner` is then constructed from the raw child key bytes via Alloy.

Solana does not use BIP-32. Its standard is to take the first 32 bytes of the seed directly as an Ed25519 signing key, which is what `ed25519-dalek` does here.

### Solana: no SDK, raw JSON-RPC

Due to dependency conflicts with `alloy` (see Evaluation), `solana-sdk` was dropped in favour of constructing transaction message bytes by hand, signing with `ed25519-dalek`, and submitting via a raw `sendTransaction` JSON-RPC call. For a simple SOL transfer the wire format is manageable, and it keeps rudi as a single self-contained binary.

### Session management

Rather than decrypt the wallet on every command, a `Session` struct holds the seed in a `Vec<u8>` and enforces two independent timeouts: an inactivity timeout (5 minutes, reset on each operation) and an absolute lifetime cap (1 hour). On expiry or drop, the seed bytes are overwritten via `zeroize`. This keeps the attack window short while avoiding repeated password prompts.

### Frontend: Tauri + HTML/CSS/JS

Rust was not designed for building graphical interfaces — its strengths lie in systems programming, performance, and safety, not in UI layout and event-driven rendering. Rather than using a native Rust UI toolkit (such as egui or iced), the project uses **Tauri**, which embeds the OS webview and exposes Rust backend logic as commands callable from a standard HTML/CSS/JavaScript frontend. Tauri was chosen because it is the most popular Rust desktop framework in this space, has strong community support and documentation, and lets the frontend be written in familiar web technologies. This separation keeps the Rust code focused purely on wallet logic and cryptography while the presentation layer is handled where web tooling genuinely excels.

### Async runtime: Tokio

All network calls are async. Tokio was the natural choice given it is the de-facto standard async runtime in the Rust ecosystem and is required transitively by both Alloy and reqwest.

---

## 5. Dependencies

| Crate | Purpose |
|---|---|
| `bip39` | Mnemonic generation and validation (BIP-39) |
| `rand` | Cryptographically secure random bytes (entropy, salt) |
| `aes-gcm` | AES-256-GCM authenticated encryption for the wallet file |
| `pbkdf2` + `sha2` | Key derivation from the user's password |
| `bitcoin` | BIP-32/BIP-44 HD key derivation, Bitcoin address generation and transaction building |
| `ed25519-dalek` | Ed25519 key generation and signing for Solana |
| `alloy` | Ethereum provider, wallet, transaction construction and broadcasting (Sepolia) |
| `reqwest` | Async HTTP client — used for Solana JSON-RPC calls, Bitcoin Blockstream API, and Ethereum Blockscout history API |
| `serde` + `serde_json` | Serialization of the wallet file and JSON parsing of API responses |
| `hex` + `bs58` + `base64` | Encoding utilities for keys, addresses, and Solana transaction bytes |
| `anyhow` | Ergonomic error propagation throughout the codebase |
| `tokio` | Async runtime |
| `zeroize` | Guaranteed zeroing of sensitive memory (seed, key material) |
| `dirs` | Cross-platform home directory resolution for wallet file path |
| `once_cell` | Lazy-initialized global HTTP client and atomic request ID counter |

---

## 6. Evaluation

### What went well

The Rust type system was a genuine asset for this project. Encoding distinctions like `Direction::Sent / Received` and `Status::Pending / Success / Rejected` as enums, and wrapping keys in dedicated structs, meant that large classes of mistakes (e.g. passing a Bitcoin key to an Ethereum function) were caught at compile time rather than at runtime. The ownership model also made the security properties easier to reason about: the compiler enforces that the seed bytes cannot be silently copied, and `zeroize` on `Drop` gives a reliable guarantee that memory is cleared.

The modular layout — separating key derivation (`tokens/`), network I/O (`networks/`), and storage (`builders/`) into distinct modules — kept each component independently testable. Writing unit tests for deterministic cryptographic functions (address derivation, transaction mapping) was straightforward, and network tests could be `#[ignore]`d to keep CI fast.

### What went not so well

The most significant obstacle was the dependency conflict between `alloy` (for Ethereum) and `solana-sdk` / `solana-client` (for Solana). Both libraries pull in large dependency trees, and several transitive crates — particularly around TLS, async runtimes, and protobuf — required mutually incompatible versions that Cargo could not reconcile. The two options were:

1. **Separate workspaces / processes** — isolate each SDK in its own binary and communicate over IPC or a subprocess interface.
2. **Replace `solana-sdk` with manual JSON-RPC** — drop the SDK entirely and implement SOL transfers by constructing the transaction bytes manually using `ed25519-dalek` and `reqwest`.

Option 2 was chosen because it kept rudi as a single binary without inter-process complexity. The downside is that the raw transaction layout is hand-coded byte-by-byte, which is more fragile than using the official SDK. For a production wallet this would be untenable, but for the scope of this project it was the more pragmatic path.

### Rust for larger projects

Compared to languages like Python, Go, or TypeScript, implementing a project of this scale in Rust feels noticeably more demanding in the early phases. The borrow checker and the requirement to handle every error path explicitly slow down the initial scaffolding. However, once the types and ownership are in place, refactoring becomes surprisingly safe — moving a function, changing a struct field, or swapping a dependency tends to surface every affected call site immediately at compile time.

The ecosystem maturity varies. Ethereum (Alloy) and Bitcoin are well-served by high-quality, well-documented crates. Solana's Rust crates are heavyweight and not designed for composability with the rest of the ecosystem, which is where the dependency conflict arose. Overall, Rust is a strong choice for systems that handle key material and money — the language's guarantees align well with the security requirements of a crypto wallet — but the steeper learning curve and longer compile times are real costs that a team should budget for. Compared to other languages, Rust is also blazing fast at runtime: it minimises abstraction levels and compiles down to machine code with no garbage collector or virtual machine in the way, meaning the performance overhead of running a wallet operation in Rust is negligible compared to the same logic in Python, Go, or the JVM.
