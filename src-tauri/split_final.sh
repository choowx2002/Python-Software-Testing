#!/bin/bash

# Extract env.rs
sed -n '75,330p' src/commands.rs > src/commands/env.rs
sed -n '1781,1835p' src/commands.rs >> src/commands/env.rs

# Extract execution.rs
sed -n '331,838p' src/commands.rs > src/commands/execution.rs
sed -n '991,1249p' src/commands.rs >> src/commands/execution.rs

# Extract suites.rs
sed -n '839,990p' src/commands.rs > src/commands/suites.rs

# Extract generation.rs
sed -n '1250,1780p' src/commands.rs > src/commands/generation.rs

# Extract files.rs
sed -n '1836,1924p' src/commands.rs > src/commands/files.rs

# Extract coverage.rs
sed -n '1925,2642p' src/commands.rs > src/commands/coverage.rs

# Extract db_commands.rs
sed -n '2643,2765p' src/commands.rs > src/commands/db_commands.rs

echo "Done extracting modules."
