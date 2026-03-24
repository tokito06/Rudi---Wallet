# Team
Oleksii Rusaniuk, Vitalii Diurd

# Introduction to our idea
## Idea
A crypto wallet that operates with Ethereum, Bitcoin, and Solana  - **RuDi** The main idea of this project is to create a unified tool where users can securely store, send, receive, and manage their cryptocurrencies without needing separate wallets for each network.
## What problems does it solve:
+ Most users need separate wallets for Bitcoin, Ethereum, and Solana. This creates confusion and inefficiency. RuDi simplifies everything into a single, intuitive interface.
+ Managing assets across multiple apps makes it harder to track balances, transactions, and portfolio performance. RuDi provides a centralized dashboard for all assets.
+ Using multiple wallets increases the risk of losing private keys or falling victim to phishing attacks. RuDi reduces exposure by consolidating management into one secure environment with strong encryption and protection features.
## What do we hope to learn
+ **Systems programming with Rust**
Learn how to build high-performance and memory-safe applications using Rust, including ownership, borrowing, and concurrency models.
+ **Secure cryptography implementation**
Understand how cryptographic primitives work in practice (key generation, signing, hashing) and how to safely implement them in a wallet environment.
+ **Blockchain integration**
Gain experience working with multiple blockchain protocols (Bitcoin, Ethereum, Solana), including how transactions are created, signed, and broadcasted.
+ **Backend & API design**
Learn how to design scalable and secure APIs (e.g., for blockchain communication, price tracking, or transaction relaying).
+ **Error handling and reliability**
Develop robust systems that safely handle failures, network issues, and edge cases without crashing or exposing vulnerabilities.
+ **Building browser extensions with Rust**
Understand how to write programs with Rust and integrate it into a browser extension environment.
## Requirements
+ Support multi-chain functionality for Bitcoin, Ethereum, and Solana within a single application.
+ Allow users to create, import, and export wallets using a secure seed phrase (HD wallet support).
+ Enable core wallet operations: send, receive, and view transaction history for all supported blockchains.
+ Provide secure private key management, including encryption and safe local storage.
+ Implement transaction signing for each blockchain according to its protocol.
+ Display a unified portfolio dashboard showing balances across all networks.
+ Integrate real-time data (e.g., balances, transaction status, basic price info).
+ Support connection to decentralized applications (dApps), at least for Ethereum and Solana.
+ Ensure basic security features such as password protection, input validation, and protection against common attacks (e.g., phishing or malicious scripts).
+ Provide a simple and intuitive user interface (browser extension format).
+ Handle network communication through appropriate RPC endpoints for each blockchain.
+ Ensure the application is stable and responsive, with proper error handling for failed transactions or network issues.
## Dependencies
- Rust toolchain
- tokio
- serde
- serde_json
- reqwest
- ethers-rs
- rust-bitcoin
- solana-sdk
- solana-client
- bip39
- bip32
- hdpath
- secp256k1
- ring
- aes-gcm
- wasm-bindgen
- wasm-pack
- yew
- leptos
- web-sys
- js-sys
- dotenv
- log
- env_logger
