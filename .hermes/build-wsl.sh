#!/bin/bash
cd ~/edgecrab-build
cp /mnt/c/Users/puske/dev/edgecrab/crates/edgecrab-cli/src/auth_cmd.rs ~/edgecrab-build/crates/edgecrab-cli/src/auth_cmd.rs 2>/dev/null
cp /mnt/c/Users/puske/dev/edgecrab/crates/edgecrab-tools/src/tools/skills_sync.rs ~/edgecrab-build/crates/edgecrab-tools/src/tools/skills_sync.rs 2>/dev/null
cp /mnt/c/Users/puske/dev/edgecrab/crates/edgecrab-tools/src/tools/web/search/error.rs ~/edgecrab-build/crates/edgecrab-tools/src/tools/web/search/error.rs 2>/dev/null
CARGO_BUILD_JOBS=2 ~/.cargo/bin/cargo +stable build --release --target x86_64-pc-windows-gnu --no-default-features -p edgecrab-cli 2>&1 | grep -E "^error|^   Compiling|FINISHED|EXIT|===|\.exe" | tail -20
echo "EXIT: $?"
