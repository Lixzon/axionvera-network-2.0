# AxionVault Events Documentation — v1

This document describes the standardized event system for AxionVault, including event schemas, topic design, indexing strategy, and compatibility considerations.

## Event Design Principles

1. **Two-Topic Design**: Every event uses `(Protocol, Action)` as its topic tuple for efficient filtering.
2. **Event Versioning**: Every event payload includes an `event_version: u32` field (currently `1`) for schema evolution.
3. **Timestamps**: Every event includes a `timestamp: u64` field set from the ledger timestamp.
4. **Consistent Naming**: All action symbols use lowercase with underscores (e.g., `deposit`, `admin_prp`, `asset_dep`).

## Event Types

| Action Symbol | Event Struct | Description |
|--------------|--------------|-------------|
| `init` | `InitializeEvent` | Emitted when the vault is initialized |
| `deposit` | `DepositEvent` | Emitted when a user deposits funds |
| `withdraw` | `WithdrawEvent` | Emitted when a user withdraws funds |
| `distrib` | `DistributeEvent` | Emitted when rewards are distributed |
| `claim` | `ClaimEvent` | Emitted when a user claims rewards |
| `lock` | `LockEvent` | Emitted when funds are locked |
| `unlock` | `UnlockEvent` | Emitted when funds are unlocked |
| `admin_prp` | `AdminTransferProposedEvent` | Emitted when an admin transfer is proposed |
| `adm_acpt` | `AdminTransferAcceptedEvent` | Emitted when an admin transfer is accepted |
| `upgrade` | `UpgradeEvent` | Emitted when the contract is upgraded |
| `pause` | `PauseEvent` | Emitted when the contract is paused |
| `unpause` | `UnpauseEvent` | Emitted when the contract is unpaused |
| `asset_add` | `AssetAddedEvent` | Emitted when a new asset is added |
| `asset_dep` | `AssetDepositEvent` | Emitted when a user deposits an asset |
| `asset_wd` | `AssetWithdrawEvent` | Emitted when a user withdraws an asset |
| `ast_dist` | `AssetDistributeEvent` | Emitted when asset rewards are distributed |
| `asset_clm` | `AssetClaimEvent` | Emitted when a user claims asset rewards |

## Topic Structure

All events use exactly two topics for Soroban event filtering:

```
Topic 1: Symbol("AxVault")     — Protocol identifier
Topic 2: Symbol("<action>")       — Action identifier (see table above)
```

This design allows indexers to rapidly filter by:
- Protocol identifier for vault-specific events
- Action type for specific state changes

## Event Schemas

All structs are defined in `contracts/events/src/lib.rs` and shared via the `axionvera-events` crate.

### Common Fields

| Field | Type | Description |
|-------|------|-------------|
| `event_version` | `u32` | Schema version (currently `1`) |
| `timestamp` | `u64` | Ledger timestamp at emission |

### InitializeEvent

```rust
struct InitializeEvent {
    event_version: u32,
    admin: Address,
    deposit_token: Address,
    reward_token: Address,
    timestamp: u64,
}
```

### DepositEvent

```rust
struct DepositEvent {
    event_version: u32,
    user: Address,
    amount: i128,
    timestamp: u64,
}
```

### WithdrawEvent

```rust
struct WithdrawEvent {
    event_version: u32,
    user: Address,
    amount: i128,
    remaining_balance: i128,
    timestamp: u64,
}
```

### DistributeEvent

```rust
struct DistributeEvent {
    event_version: u32,
    caller: Address,
    amount: i128,
    timestamp: u64,
}
```

### ClaimEvent

```rust
struct ClaimEvent {
    event_version: u32,
    user: Address,
    amount: i128,
    timestamp: u64,
}
```

### LockEvent

```rust
struct LockEvent {
    event_version: u32,
    user: Address,
    amount: i128,
    unlock_timestamp: u64,
    timestamp: u64,
}
```

### UnlockEvent

```rust
struct UnlockEvent {
    event_version: u32,
    user: Address,
    amount: i128,
    timestamp: u64,
}
```

### AdminTransferProposedEvent

```rust
struct AdminTransferProposedEvent {
    event_version: u32,
    current_admin: Address,
    pending_admin: Address,
    timestamp: u64,
}
```

### AdminTransferAcceptedEvent

```rust
struct AdminTransferAcceptedEvent {
    event_version: u32,
    previous_admin: Address,
    new_admin: Address,
    timestamp: u64,
}
```

### UpgradeEvent

```rust
struct UpgradeEvent {
    event_version: u32,
    admin: Address,
    new_wasm_hash: BytesN<32>,
    timestamp: u64,
}
```

### PauseEvent

```rust
struct PauseEvent {
    event_version: u32,
    admin: Address,
    timestamp: u64,
}
```

### UnpauseEvent

```rust
struct UnpauseEvent {
    event_version: u32,
    admin: Address,
    timestamp: u64,
}
```

### AssetAddedEvent

```rust
struct AssetAddedEvent {
    event_version: u32,
    asset: Address,
    timestamp: u64,
}
```

### AssetDepositEvent

```rust
struct AssetDepositEvent {
    event_version: u32,
    user: Address,
    asset: Address,
    amount: i128,
    timestamp: u64,
}
```

### AssetWithdrawEvent

```rust
struct AssetWithdrawEvent {
    event_version: u32,
    user: Address,
    asset: Address,
    amount: i128,
    remaining_balance: i128,
    timestamp: u64,
}
```

### AssetDistributeEvent

```rust
struct AssetDistributeEvent {
    event_version: u32,
    caller: Address,
    asset: Address,
    amount: i128,
    timestamp: u64,
}
```

### AssetClaimEvent

```rust
struct AssetClaimEvent {
    event_version: u32,
    user: Address,
    asset: Address,
    amount: i128,
    timestamp: u64,
}
```

## On-Chain Indexing

An on-chain event log is maintained via `axionvera-core` (`contracts/core/`). Key features:

### EventLogEntry

```rust
struct EventLogEntry {
    action: Symbol,
    user: Option<Address>,
    asset: Option<Address>,
    amount: i128,
    timestamp: u64,
    ledger: u32,
}
```

### Indexing Behavior

- **Global Event Log**: Up to 200 most recent events stored in instance storage
- **Per-User Event Log**: Up to 50 most recent events per user in persistent storage
- **Interacting Users Set**: A Map of all unique user addresses that have interacted
- **Automatic Indexing**: Events with user addresses are automatically indexed via `index_event()` in `core/src/lib.rs`
- **TTL Management**: Instance storage TTL is bumped on each index operation

### Query Functions

| Function | Description |
|----------|-------------|
| `get_global_event_log(e)` | Returns the global event log (Vec<EventLogEntry>) |
| `get_user_event_log(e, user)` | Returns events for a specific user (Vec<EventLogEntry>) |
| `get_interacting_users(e)` | Returns all unique user addresses |

## Database Schema (Off-Chain Indexer)

Events are stored in the `events` table with the following columns:

| Column | Type | Description |
|--------|------|-------------|
| `id` | SERIAL | Primary key |
| `event_id` | TEXT | Unique event ID |
| `ledger_sequence` | INTEGER | Stellar ledger number |
| `contract_id` | TEXT | Contract address |
| `event_type` | TEXT | Event type |
| `protocol` | TEXT | Protocol identifier (always "AxionVault") |
| `action` | TEXT | Action type (e.g., "deposit", "withdraw") |
| `user_address` | TEXT | User address (if applicable) |
| `asset_address` | TEXT | Asset address (if applicable) |
| `amount` | NUMERIC | Amount (if applicable) |
| `timestamp` | BIGINT | Unix timestamp from the event |
| `event_version` | INTEGER | Event schema version |
| `data` | JSONB | Full event payload as JSON |
| `created_at` | TIMESTAMPTZ | When the event was indexed |

## Compatibility Considerations

### Topic Change (Single → Two Topics)

Events that previously used a single-topic design (`AdminProp`, `AdminAcpt`, `Upgrade`, `AssetAdd`) now use the standard two-topic `(AxVault, <action>)` pattern. Off-chain indexers **must** be updated to filter on `topic[1]` instead of `topic[0]` for these events.

### New Pause/Unpause Events

`PauseEvent` and `UnpauseEvent` are new events emitted when the contract is paused or unpaused. Indexers should handle these gracefully.

### Event Version Field

All event payloads now include `event_version: u32` to support future schema changes. Consumers should validate this field and handle unknown versions gracefully.

### Action Symbol Renames

| Old Symbol | New Symbol |
|-----------|------------|
| `AxionVault` (protocol) | `AxVault` (protocol) |
| `Initialize` | `init` |
| `Deposit` | `deposit` |
| `Withdraw` | `withdraw` |
| `Distribute` | `distrib` |
| `Claim` | `claim` |
| `Lock` | `lock` |
| `Unlock` | `unlock` |
| `AdminProp` | `admin_prp` |
| `AdminAcpt` | `adm_acpt` |
| `Upgrade` | `upgrade` |
| `AssetAdd` | `asset_add` |
| `AssetDep` | `asset_dep` |
| `AssetWith` | `asset_wd` |
| `AssetDist` | `ast_dist` |
| `AssetClm` | `asset_clm` |

## Indexer API

The off-chain indexer runs in the network node and:
1. Polls the Soroban RPC for new events every 5 seconds
2. Uses topic filter `["AxVault"]` to capture all vault events
3. Parses the action from `topic[1]` for event type identification
4. Stores events in the PostgreSQL database
5. Tracks progress in the `indexer_state` table
6. Provides idempotent processing (events are not duplicated)

## Querying Events

### Query All Events

```sql
SELECT * FROM events ORDER BY timestamp DESC LIMIT 100;
```

### Query Events by User

```sql
SELECT * FROM events WHERE user_address = 'GABC...XYZ' ORDER BY timestamp DESC;
```

### Query Events by Type

```sql
SELECT * FROM events WHERE action = 'deposit' ORDER BY timestamp DESC;
```

### Query Events by Date Range

```sql
SELECT * FROM events WHERE timestamp >= 1710000000 AND timestamp <= 1720000000;
```

## Library Crates

The event system is split across three crates:

| Crate | Path | Purpose |
|-------|------|---------|
| `axionvera-events` | `contracts/events/` | Event struct definitions, action symbols, DataKey for indexing |
| `axionvera-core` | `contracts/core/` | On-chain event indexing (EventLogEntry, index_event, query functions) |
| `axionvera-interfaces` | `contracts/interfaces/` | `VaultEventEmitter` trait for standardized emission |

## Future Enhancements

Planned improvements:
- Parse XDR event values into structured JSON in the off-chain indexer
- Add more specific query endpoints (gRPC and HTTP)
- Add WebSocket subscriptions for real-time events
- Add event analytics dashboards
