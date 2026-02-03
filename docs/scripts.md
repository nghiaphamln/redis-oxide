# Lua Scripts

Execute Lua scripts on the Redis server for atomic, high-performance operations.

## Basic Script Execution

```rust
use redis_oxide::{Client, ConnectionConfig, Script};

let script = r#"
    local key = KEYS[1]
    local value = ARGV[1]
    local current = redis.call('GET', key)
    if current then
        return current .. ':' .. value
    else
        return value
    end
"#;

let result: String = client.eval(script, vec!["mykey"], vec!["newvalue"]).await?;
```

## Script with Multiple Arguments

```rust
let script = r#"
    local key = KEYS[1]
    local increment = tonumber(ARGV[1])
    local max = tonumber(ARGV[2])

    local current = redis.call('GET', key) or '0'
    local value = tonumber(current) + increment

    if max and value > max then
        return 'EXCEEDED'
    end

    redis.call('SET', key, value)
    return tostring(value)
"#;

let result = client.eval(script, vec!["counter"], vec!["5", "100"]).await?;
```

## Script with Multiple Keys

```rust
let script = r#"
    local key1 = KEYS[1]
    local key2 = KEYS[2]

    local val1 = redis.call('GET', key1) or '0'
    local val2 = redis.call('GET', key2) or '0'

    return val1 + val2
"#;

let result: i64 = client.eval(script, vec!["key1", "key2"], vec![]).await?;
```

## EVALSHA

For scripts that have been cached, use EVALSHA to avoid re-sending the script:

```rust
// First, get the SHA1 hash of a script
let script = "return redis.call('GET', KEYS[1])";
let sha: String = client.script_load(script).await?;

// Use EVALSHA with the hash
let result: Option<String> = client.evalsha(&sha, vec!["mykey"], vec![]).await?;
```

## Script Management Commands

```rust
// Check if script exists in cache
let exists: bool = client.script_exists(vec!["sha1_hash1", "sha1_hash2"]).await?;

// Remove all scripts from cache
client.script_flush().await?;

// Kill currently running script
client.script_kill().await?;
```

## Script Utility Functions

```rust
// Return information about the last HSET operation
// Returns: (field, value, exists_before)
// status: "OK" if new field, "0" if updated existing field

// Return Redis version
// Returns: Version string like "7.0.0"

// Sleep for specified seconds (use for testing)
// Blocks the server, use sparingly
```

## Best Practices

1. **Keep scripts short**: Shorter scripts execute faster
2. **Use KEYS and ARGV properly**: KEYS for key names, ARGV for other arguments
3. **Cache scripts**: Use EVALSHA for repeated script execution
4. **Avoid expensive operations**: Scripts block the server
5. **Test scripts**: Use redis-cli before embedding in code

## Performance Tips

- Scripts are executed atomically on the server
- Reduce network round-trips for complex operations
- Use for operations that would require multiple round-trips
- Avoid in high-throughput scenarios with simple operations

## Common Script Patterns

### Atomic Counter with Cap

```lua
local key = KEYS[1]
local increment = tonumber(ARGV[1])
local cap = tonumber(ARGV[2])

local current = tonumber(redis.call('GET', key) or '0')
local new_value = current + increment

if new_value > cap then
    return -1
end

redis.call('SET', key, new_value)
return new_value
```

### List Processing

```lua
local key = KEYS[1]
local count = tonumber(ARGV[1])

local items = {}
for i = 1, count do
    local item = redis.call('RPOP', key)
    if not item then break end
    table.insert(items, item)
end

return items
```

### Hash Field Update

```lua
local key = KEYS[1]
local field = ARGV[1]
local value = ARGV[2]
local only_if_exists = ARGV[3] == '1'

if only_if_exists then
    return redis.call('HSETNX', key, field, value)
else
    return redis.call('HSET', key, field, value)
end
```
