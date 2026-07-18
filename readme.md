# Rust Bitcoin Core RPC CLI (`btc-cli`)

`btc-cli` is a modern, asynchronous command-line interface (CLI) written in Rust. It enables developers and operators to interact with a Bitcoin Core node (such as a local **Regtest** node managed via **Polar**) using the Bitcoin Core JSON-RPC API.

This project translates complex JSON-RPC communication into a simple, friendly command-line experience with built-in configurations, robust error diagnostics, and a generic RPC executor.

---

## 📖 Table of Contents
1. [Concepts for Non-Technical Users](#1-concepts-for-non-technical-users)
2. [Quick Start & Polar Setup](#2-quick-start--polar-setup)
3. [Configuration & Customization](#3-configuration--customization)
4. [Solving the "No Wallet Loaded" Error (Important)](#4-solving-the-no-wallet-loaded-error-important)
5. [Command Walkthrough & Terminal Outputs](#5-command-walkthrough--terminal-outputs)
6. [System Architecture & Developer Notes](#6-system-architecture--developer-notes)
7. [Running Automated Tests](#7-running-automated-tests)

---

## 1. Concepts for Non-Technical Users

If you are new to Bitcoin development or running node operations, here is a quick guide to some of the terms used in this tool:

* **Regtest (Regression Test Mode)**: A private, local Bitcoin blockchain configuration. Unlike the real Bitcoin network (Mainnet), you can instantly mine blocks, generate test Bitcoins, and test applications in a sandbox environment without spending real money.
* **Polar**: A user-friendly desktop application that lets you set up and run local Bitcoin, Lightning, and Bitcoin Core networks inside Docker containers with a single click.
* **JSON-RPC**: A lightweight protocol used by the CLI to talk to your Bitcoin node. The CLI sends a structured question (e.g., "What is the wallet balance?") and the node returns a structured answer.
* **Wallet**: An active Bitcoin Core container wallet file containing keys. Bitcoin Core allows running multiple wallets. A wallet must be explicitly **created** and **loaded** in the node before you can query its balances or addresses.

---

## 2. Quick Start & Polar Setup

### Step A: Prerequisites
1. **Docker Desktop**: Ensure Docker is installed and running on your computer. Download it from [docker.com](https://www.docker.com/).
2. **Polar**: Download and install Polar from [lightningpolar.com](https://lightningpolar.com/).
3. **Rust Compiler**: Ensure you have Rust installed. You can install it via [rustup.rs](https://rustup.rs/).

### Step B: Launching a Regtest Node in Polar
1. Open Polar and click **Create a Network**.
2. Give your network a name (e.g., `regtest-network`) and add at least one **Bitcoin Core** node.
3. Click **Create Network**, then click the green **Start** button in the top right to launch the containers.
4. Click on the Bitcoin Core node and navigate to the **Connect** tab.
5. Take note of the **RPC Host (URL)**, **Username**, and **Password**.

### Step C: Bootstrapping Configuration
Create a file named `config.toml` in the project root:
```bash
# You can copy config.example.toml as a baseline
cp config.example.toml config.toml
```

Edit your `config.toml` to match your Polar credentials:
```toml
rpc_url = "http://127.0.0.1:18443"
rpc_user = "polaruser"
rpc_password = "polarpass"
wallet = "wallet1"

# Additional Polar connection details for your reference
p2p_host = "tcp://127.0.0.1:19444"
zmq_block_host = "tcp://127.0.0.1:28334"
zmq_transaction_host = "tcp://127.0.0.1:29335"
```

---

## 3. Configuration & Customization

`btc-cli` resolves configuration inputs using a hierarchy of priorities. If a configuration parameter is present in multiple places, the higher-priority option wins:

$$\text{Command-Line Flags (CLI)} > \text{Environment Variables} > \text{TOML Configuration File} > \text{Built-in Defaults}$$

### Available Configuration Settings

| Parameter | TOML File Key | Environment Variable | CLI Flag | Default Value |
|---|---|---|---|---|
| **RPC Host Endpoint** | `rpc_url` | `BTC_RPC_URL` | `--rpc-url` | `http://127.0.0.1:18443` |
| **RPC Username** | `rpc_user` | `BTC_RPC_USER` | `--rpc-user` | *None (Required)* |
| **RPC Password** | `rpc_password` | `BTC_RPC_PASSWORD` | `--rpc-password` | *None (Required)* |
| **Default Wallet** | `wallet` | `BTC_RPC_WALLET` | `--wallet` | `wallet1` |
| **Custom TOML Path** | *N/A* | *N/A* | `--config` | `./config.toml` |

---

## 4. Solving the "No Wallet Loaded" Error (Important)

> [!IMPORTANT]
> When running wallet commands like `wallet-info` or `balance` on a brand new Polar node, you may see the following error:
> 
> ```
> Error: wallet error: no wallet is loaded — run 'btc-cli rpc loadwallet <name>' or 'btc-cli rpc createwallet <name>'
> ```
> 
> This happens because Bitcoin Core has not created or loaded the wallet file in memory yet. You can resolve this immediately by calling the node management commands using our built-in generic RPC passthrough.

### Resolution Step 1: Create a Wallet
If this is the first time you are using this wallet name, run the `createwallet` command:
```bash
cargo run -- rpc createwallet "wallet1"
```
**Expected Terminal Output:**
```json
{
  "name": "wallet1",
  "warning": ""
}
```

### Resolution Step 2: Load a Wallet
If the wallet is already created but was unloaded (e.g. after restarting the Polar node), load it:
```bash
cargo run -- rpc loadwallet "wallet1"
```
**Expected Terminal Output:**
```json
{
  "name": "wallet1",
  "warning": ""
}
```

---

## 5. Command Walkthrough & Terminal Outputs

Here are examples of all primary subcommands supported by `btc-cli`:

### 📊 `blockchain-info`
Queries the blockchain state and verification difficulty.

```bash
cargo run -- blockchain-info
```
**Terminal Output:**
```
Chain:                regtest
Blocks:                150
Headers:               150
Difficulty:            0.00000000465654
Verification progress: 100.00%
```

---

### 💳 `wallet-info`
Displays detailed information about the active wallet, including balances and transaction counts.

```bash
cargo run -- --wallet wallet1 wallet-info
```
**Terminal Output:**
```
Wallet:               wallet1
Balance:              50.00000000 BTC
Unconfirmed balance:  0.00000000 BTC
Transactions:         3
```

---

### 💰 `balance`
Fetches and prints the balance of the configured wallet.

* **Default confirmed balance:**
  ```bash
  cargo run -- --wallet wallet1 balance
  ```
  **Terminal Output:**
  ```
  50.00000000 BTC
  ```

* **Including unconfirmed balance:**
  ```bash
  cargo run -- --wallet wallet1 balance --include-unconfirmed
  ```
  **Terminal Output:**
  ```
  Confirmed:    50.00000000 BTC
  Unconfirmed:  12.50000000 BTC
  ```

---

### 🔑 `new-address`
Generates a new receiving address for the wallet to receive testnet funds.

```bash
cargo run -- --wallet wallet1 new-address --label "donation-address" --address-type bech32
```
**Terminal Output:**
```
bcrt1q7y7epxgrs6vsnq272j9a6nuf6z4f9l5s3z3wpe
```

---

### ⚡ Generic `rpc` Passthrough
Executes any arbitrary Bitcoin Core RPC method directly. Arguments are dynamically coerced to correct JSON types (booleans, integers, lists, or strings) automatically.

* **Retrieve block hash at height 0:**
  ```bash
  cargo run -- rpc getblockhash 0
  ```
  **Terminal Output:**
  ```
  "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
  ```

* **Coercing parameter types (Integer and Boolean):**
  ```bash
  cargo run -- rpc getblock "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206" 1
  ```
  **Terminal Output:**
  ```json
  {
    "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
    "confirmations": 151,
    "height": 0,
    "version": 1,
    "versionHex": "00000001",
    "merkleroot": "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
    "time": 1296688602,
    "mediantime": 1296688602,
    "nonce": 2,
    "bits": "207fffff",
    "difficulty": 4.6565423739069247e-10,
    "chainwork": "0000000000000000000000000000000000000000000000000000000000000002",
    "nTx": 1,
    "nextblockhash": "0f05f2c41c7b1f3c3a4f328bf5beb436012afca590b1a11466e220600000000"
  }
  ```

---

## 6. System Architecture & Developer Notes

### Core Architecture
- **Async Runtime**: Powered by `tokio` (multi-threaded task execution scheduler).
- **HTTP Engine**: `reqwest` client sends asynchronous, non-blocking HTTP POST payloads with basic authentication.
- **Serialization**: `serde` and `serde_json` handle payload building, response envelopes, and structural parsing.

### Endpoint Routing Logic
To access wallet-scoped resources (e.g. `getwalletinfo`), the HTTP URL must append `/wallet/<name>` to the base path. However, wallet administration commands (such as `createwallet`, `loadwallet`, `listwallets`) must be directed at the root `/` endpoint. 

`btc-cli` implements a routing filter in `src/rpc.rs`:
```rust
let is_node_command = matches!(
    method,
    "createwallet" | "loadwallet" | "unloadwallet" | "listwallets" | "listwalletdir"
);
match &self.wallet {
    Some(w) if !is_node_command => format!("{}/wallet/{}", base_url, w),
    _ => base_url,
}
```

### Positional Argument Coercion
CLI input is initially read as strings. To avoid asking users to manually format complex JSON strings on the command line, the CLI attempts to parse each raw input argument using `serde_json::from_str`. If parsing fails, it defaults to a standard string format. This permits commands such as:
- `rpc getblockhash 0` (parsed as integer `0` instead of string `"0"`)
- `rpc getbalances true` (parsed as boolean `true`)
- `rpc getblock <hash> false` (parsed as string, then boolean)

### Error Handling Diagnostics
The CLI safely maps common failure types:
- **Connection Refused**: Automatically diagnostic of offline nodes: `could not reach node at http://127.0.0.1:18443 — connection refused`.
- **401 Unauthorized**: Maps credentials failures: `authentication failed for http://127.0.0.1:18443 — check RPC user/password`.
- **RPC Code -18**: Catches unloaded wallets and suggests the correct recovery commands.

---

## 7. Running Automated Tests

To maintain codebase safety and guarantee API mappings are correct without invoking a live Bitcoin node, the test suite utilizes `wiremock` to simulate real JSON-RPC responses.

Run all tests:
```bash
cargo test
```

### Active Logging (Debug Mode)
To view HTTP transactions, route logs, and timings in real-time, launch commands prefixing the `RUST_LOG` environment variable:
```bash
# On Linux/macOS:
RUST_LOG=debug cargo run -- blockchain-info

# On Windows PowerShell:
$env:RUST_LOG="debug"; cargo run -- blockchain-info
```
