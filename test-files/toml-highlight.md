# TOML Syntax Highlighting

Test file for TOML tree-sitter highlighting regression testing.

```toml
# Comment - should be blue
[package]
name = "my-app"
version = "0.1.0"
edition = "2024"
authors = ["Alice <alice@example.com>", "Bob"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", optional = true }

[profile.release]
lto = true
opt-level = 3
debug = false

# Array of tables
[[bin]]
name = "app"
path = "src/main.rs"

[[test]]
name = "integration"
path = "tests/integration.rs"

[database]
server = "192.168.1.1"
ports = [8000, 8001, 8002]
enabled = true
connection_max = 5000
pi = 3.14159
negative = -42
hex = 0xDEADBEEF
float_special = inf

[dates]
created = 2024-01-15T10:30:00Z
local_date = 2024-01-15
local_time = 10:30:00
offset = 2024-01-15T10:30:00+09:00

[dotted.keys.example]
value = "nested via dots"

[inline]
point = { x = 1, y = 2 }
colors = ["red", "green", "blue"]
```

Expected highlighting:
- Comments: blue
- Section headers `[package]`: cyan (type)
- Keys: blue (property)
- Strings: magenta
- Booleans (true/false): yellow (constant)
- Numbers (integers, floats): yellow (constant)
- Dates/times: magenta (string.special)
- Brackets `[]`, `{}`: blue
- Equals `=`: red (operator)
- Commas, dots: blue (delimiter)
