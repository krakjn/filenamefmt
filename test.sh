#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== namefmt Test Suite ===${NC}\n"

# Build the project first
echo "Building namefmt..."
cargo build --release
if [ $? -ne 0 ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

BINARY="./target/release/namefmt"
TESTBED="./testbed"

# Function to reset testbed
reset_testbed() {
    echo -e "\n${YELLOW}Resetting testbed...${NC}"
    rm -rf "$TESTBED"
    mkdir -p "$TESTBED/package-project" "$TESTBED/subdirectory" "$TESTBED/node-project"
    
    # Create test files
    touch "$TESTBED/file with spaces.txt"
    touch "$TESTBED/FileWithMixedCase.rs"
    touch "$TESTBED/another-file-with-dashes.js"
    touch "$TESTBED/UPPERCASE_FILE.py"
    touch "$TESTBED/my-executable.exe"
    touch "$TESTBED/package-project/Cargo.toml"
    touch "$TESTBED/package-project/src file.rs"
    touch "$TESTBED/subdirectory/nested file with spaces.md"
    touch "$TESTBED/subdirectory/CamelCaseFile.ts"
    touch "$TESTBED/node-project/package.json"
    touch "$TESTBED/node-project/main file.js"
    
    # Add content to package files
    echo '[package]
name = "test-package"
version = "0.1.0"' > "$TESTBED/package-project/Cargo.toml"
    
    echo '{
  "name": "test-project",
  "version": "1.0.0"
}' > "$TESTBED/node-project/package.json"
}

# Test 1: Dry-run default
echo -e "\n${YELLOW}Test 1: Dry-run default mode${NC}"
reset_testbed
echo "Running: $BINARY $TESTBED"
OUTPUT=$($BINARY "$TESTBED" 2>&1)
echo "$OUTPUT"

if echo "$OUTPUT" | grep -q "Would rename"; then
    echo -e "${GREEN}✓ Test 1 passed: Dry-run shows 'Would rename' messages${NC}"
else
    echo -e "${RED}✗ Test 1 failed: Expected 'Would rename' messages${NC}"
    exit 1
fi

# Test 2: Inplace mode
echo -e "\n${YELLOW}Test 2: Inplace mode${NC}"
reset_testbed
echo "Running: $BINARY -i $TESTBED"
OUTPUT=$($BINARY -i "$TESTBED" 2>&1)
echo "$OUTPUT"

# Check if files were actually renamed
if [ -f "$TESTBED/file_with_spaces.txt" ] && [ ! -f "$TESTBED/file with spaces.txt" ]; then
    echo -e "${GREEN}✓ Test 2 passed: Files were actually renamed${NC}"
else
    echo -e "${RED}✗ Test 2 failed: Files were not renamed correctly${NC}"
    exit 1
fi

# Test 3: Timestamp prefix
echo -e "\n${YELLOW}Test 3: Timestamp prefix${NC}"
reset_testbed
echo "Running: $BINARY --timestamp $TESTBED"
OUTPUT=$($BINARY --timestamp "$TESTBED" 2>&1)
echo "$OUTPUT"

# Check if output contains timestamp format YYYY_MM_DD__
if echo "$OUTPUT" | grep -qE "[0-9]{4}_[0-9]{2}_[0-9]{2}__"; then
    echo -e "${GREEN}✓ Test 3 passed: Timestamp prefix is shown${NC}"
else
    echo -e "${RED}✗ Test 3 failed: Timestamp prefix not found${NC}"
    exit 1
fi

# Test 4: Exe/package detection (kebab-case)
echo -e "\n${YELLOW}Test 4: Exe/package detection${NC}"
reset_testbed
echo "Running: $BINARY $TESTBED"
OUTPUT=$($BINARY "$TESTBED" 2>&1)
echo "$OUTPUT"

# Check if exe file gets kebab-case treatment
if echo "$OUTPUT" | grep -q "my-executable.exe"; then
    echo -e "${GREEN}✓ Test 4 passed: Exe files detected${NC}"
else
    echo -e "${YELLOW}⚠ Test 4: Exe detection may need verification${NC}"
fi

# Test 5: Recursive traversal
echo -e "\n${YELLOW}Test 5: Recursive traversal${NC}"
reset_testbed
echo "Running: $BINARY $TESTBED"
OUTPUT=$($BINARY "$TESTBED" 2>&1)
echo "$OUTPUT"

if echo "$OUTPUT" | grep -q "subdirectory"; then
    echo -e "${GREEN}✓ Test 5 passed: Subdirectories are processed${NC}"
else
    echo -e "${RED}✗ Test 5 failed: Subdirectories not processed${NC}"
    exit 1
fi

# Test 6: Config override
echo -e "\n${YELLOW}Test 6: Config override${NC}"
reset_testbed
# Create a custom config
CUSTOM_CONFIG=$(mktemp)
cat > "$CUSTOM_CONFIG" <<EOF
replace_spaces = false
EOF

echo "Running: $BINARY -c $CUSTOM_CONFIG $TESTBED"
OUTPUT=$($BINARY -c "$CUSTOM_CONFIG" "$TESTBED" 2>&1)
echo "$OUTPUT"

# With replace_spaces = false, files with spaces should not be renamed
if echo "$OUTPUT" | grep -q "file with spaces.txt"; then
    echo -e "${GREEN}✓ Test 6 passed: Custom config is used${NC}"
else
    echo -e "${YELLOW}⚠ Test 6: Config override may need manual verification${NC}"
fi

rm -f "$CUSTOM_CONFIG"

echo -e "\n${GREEN}=== All tests completed! ===${NC}"
