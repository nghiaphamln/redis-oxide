# Architecture

`redis-oxide` separates the public client API, command encoding, transport,
and topology routing.

- `Client` owns configuration and routes commands to standalone, Cluster, or
  Sentinel-backed pools.
- Command modules define typed command arguments, response parsing, and keys
  for Cluster routing.
- RESP2 and RESP3 codecs handle wire values; command arguments are encoded as
  bulk strings.
- The connection layer owns TCP I/O, authentication, database selection,
  protocol negotiation, and timeouts.
- Pools use either one bounded multiplexed worker or permit-guarded dedicated
  connections.

Transactions and subscribers receive dedicated connections so session state
cannot leak into unrelated requests. Cluster mode keeps slot-to-node pools;
`ASKING` and redirected commands share one physical connection.

## Request flow

1. A `Client` method builds command arguments and routing keys.
2. The client selects a standalone or slot-specific pool.
3. A connection encodes the command and waits for the RESP response.
4. The command parser converts that response into the public return type.

Redis server errors inside pipelines remain per-command `RespValue::Error`
values. Connection, timeout, and protocol failures fail the operation and make
the affected pooled connection ineligible for reuse.

The test suite covers protocol parsing, standalone integration behavior,
commands, stateful connections, Cluster, and Sentinel topology fixtures.
