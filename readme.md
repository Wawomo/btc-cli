# Rust Bitcoin Core RPC CLI (`btc-cli`)

`btc-cli` is a command-line utility built in Rust that communicates with a local Bitcoin Core Regtest node (typically provisioned using **Polar**) via the Bitcoin Core JSON-RPC interface.

---

## Features

- **Configuration Precedence**: Seamlessly reads configuration from command-line flags, environment variables, a local TOML configuration file, or built-in defaults (CLI > Env > File > Default).
- **Graceful Error Handling**: Maps standard Bitcoin Core RPC errors, connection refused exceptions, and HTTP status codes (such as `401 Unauthorized`) into clear, human-readable error messages without panics or stack traces.
- **Strongly-Typed Interfaces**: Deserializes core domain methods (`getblockchaininfo`, `getwalletinfo`, `getbalance`, and `getnewaddress`) into native Rust structs.
- **Generic RPC Passthrough**: Provides an arbitrary RPC executor subcommand (`rpc <method> [params...]`) that translates CLI positional arguments to their corresponding JSON types automatically.

---

## 1. Setting Up Polar and Bitcoin Core

### Installing Polar
1. Make sure **Docker Desktop** is installed and running on your system.
2. Download the installer for **Polar** (available for Windows, macOS, and Linux) from [lightningpolar.com](https://lightningpolar.com/).
3. Install Polar following standard procedures for your operating system.

### Creating a Regtest Node
1. Open Polar and select **Create a Network** (or the orange button in the center).
2. Choose a name for your network (e.g. `regtest-cli-demo`).
3. Add at least one **Bitcoin Core** node to the network topology.
4. Click **Create Network**, then click the **Start** button in the top right corner to spin up the containers.

### Obtaining RPC Credentials
Once the network starts:
1. Click on the **Bitcoin Core** node in Polar to open its details panel.
2. Navigate to the **Connect** tab.
3. Note the following values:
   - **RPC URL** (e.g. `http://127.0.0.1:18443` or other dynamic port)
   - **RPC Username** (e.g. `polaruser`)
   - **RPC Password** (e.g. `polarpass`)

---

## 2. Configuring the Application

`btc-cli` resolves configuration with the following precedence order:
1. **Command-Line Flags** (`--rpc-url`, `--rpc-user`, `--rpc-password`, `--wallet`, `--config`)
2. **Environment Variables** (`BTC_RPC_URL`, `BTC_RPC_USER`, `BTC_RPC_PASSWORD`, `BTC_RPC_WALLET`)
3. **Config File** (a `./config.toml` file, or a custom file targeted via `--config <path>`)
4. **Built-in Defaults** (default RPC URL is `http://127.0.0.1:18443`; username and password have no safe defaults and must be supplied).

### Configuration Template
You can bootstrap configuration by copying the template file:
```bash
cp config.example.toml config.toml
```

Update your `config.toml` file with your credentials:
```toml
rpc_url = "http://127.0.0.1:18443"
rpc_user = "polaruser"
rpc_password = "polarpass"
wallet = "wallet1"
```

*Note: `config.toml` is gitignored to prevent accidental commits of real credentials.*

---

## 3. Running the Application

Compile the project:
```bash
cargo build
```

Run subcommands directly using `cargo run -- [flags] <command>`:

### `blockchain-info`
Queries node status and displays current block count, difficulty, and verification progress:
```bash
cargo run -- blockchain-info
```
```
Chain:                regtest
Blocks:                12
Headers:               12
Difficulty:            4.656542373906925e-10
Verification progress: 100.00%
```

### `wallet-info`
Queries wallet status for a specified wallet:
```bash
cargo run -- --wallet wallet1 wallet-info
```
```
Wallet:                wallet1
Balance:               50.00000000 BTC
Unconfirmed balance:   0.00000000 BTC
Transactions:          3
```

### `balance`
Fetches and prints wallet balance:
```bash
cargo run -- --wallet wallet1 balance
```
```
50.00000000 BTC
```

With unconfirmed transactions included:
```bash
cargo run -- --wallet wallet1 balance --include-unconfirmed
```
```
Confirmed:    50.00000000 BTC
Unconfirmed:  0.00000000 BTC
```

### `new-address`
Generates a new receiving address for the wallet:
```bash
cargo run -- --wallet wallet1 new-address --label demo --address-type bech32
```
```
bcrt1qexampleaddressxxxxxxxxxxxxxxxxxxxxxx
```

### `rpc` (Generic Passthrough)
Executes arbitrary JSON-RPC methods on the node. The arguments are dynamically parsed as numbers, booleans, arrays, or objects (falling back to strings).

```bash
cargo run -- rpc getblockcount
```
```
12
```

```bash
cargo run -- rpc getblockhash 0
```
```
"0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206"
```

```bash
cargo run -- rpc getblock 0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206
```
```json
{
  "hash": "0f9188f13cb7b2c71f2a335e3a4fc328bf5beb436012afca590b1a11466e2206",
  "confirmations": 12,
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

## 4. Error Handling Walkthrough

The CLI application gracefully surfaces errors into clear user-friendly messages and exits with status code `1` under failure scenarios:

| Action | Error Message |
|---|---|
| **Wrong RPC password** | `Error: authentication failed for http://127.0.0.1:18443 — check RPC user/password` |
| **Polar node stopped** | `Error: could not reach node at http://127.0.0.1:18443 — connection refused` |
| **Typo'd RPC method** | `Error: RPC error -32601: Method not found` |
| **Bad parameter type** | `Error: RPC error -8: JSON value is not an integer` |
| **Missing/Unloaded wallet** | `Error: no wallet is loaded — run 'btc-cli rpc loadwallet <name>' or 'btc-cli rpc createwallet <name>'` |

---

## 5. Design Decisions and Assumptions

1. **Async Runtime**: Built using `tokio` and `reqwest`'s non-blocking HTTP client. This provides natural support for modern async networking, resolving the "Async implementation" bonus.
2. **Positional Parameter Coercion**: The generic `rpc` passthrough tries to parse arguments as valid JSON (objects, arrays, booleans, numbers) using `serde_json::from_str`. If parsing fails, it defaults to a JSON string. This prevents the user from having to manually format JSON quotes in the shell for basic types.
3. **Wallet Routing**: Wallet-scoped subcommands automatically query the `{rpc_url}/wallet/{name}` endpoint if a wallet is configured, resolving namespaced wallet operations.

---

## 6. Testing

The project includes an extensive test suite verifying configuration loading precedence, parameter coercion, JSON deserialization structures, and RPC transport mapping (using `wiremock` to simulate real JSON-RPC HTTP success and failure statuses without requiring a live node).

Run the tests:
```bash
cargo test
```
