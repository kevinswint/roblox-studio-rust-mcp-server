#!/bin/bash
# End-to-end test for MCP input simulation via HTTP polling
# Run this WHILE playtest is running in Studio

set -e

MCP_URL="http://localhost:44755/mcp/input"
TEST_KEY="TestKey$(date +%s)"

echo "=== MCP Input Simulation Test ==="
echo ""

# Test 1: Check endpoint is reachable
echo "[TEST 1] Checking endpoint reachability..."
RESPONSE=$(curl -s -w "%{http_code}" -o /tmp/mcp_response.json "$MCP_URL" 2>/dev/null)
if [ "$RESPONSE" != "200" ]; then
    echo "❌ FAIL: Endpoint returned HTTP $RESPONSE (expected 200)"
    echo "   Is Claude Code running with the updated MCP server?"
    exit 1
fi
echo "✅ PASS: Endpoint reachable"

# Test 2: Check response format
echo ""
echo "[TEST 2] Checking response format..."
if ! jq -e '.commands' /tmp/mcp_response.json > /dev/null 2>&1; then
    echo "❌ FAIL: Response missing 'commands' field"
    cat /tmp/mcp_response.json
    exit 1
fi
echo "✅ PASS: Valid response format"

# Test 3: Queue is being polled (should be empty if game is polling)
echo ""
echo "[TEST 3] Checking if game is polling..."
QUEUE_COUNT=$(jq -r '.count' /tmp/mcp_response.json)
if [ "$QUEUE_COUNT" != "0" ]; then
    echo "⚠️  WARNING: Queue has $QUEUE_COUNT commands - game may not be polling"
    echo "   Make sure playtest is running with MCPInputPoller script"
else
    echo "✅ PASS: Queue is empty (game is polling)"
fi

echo ""
echo "=== All basic tests passed ==="
echo ""
echo "To test full input flow:"
echo "1. Ensure playtest is running in Studio"
echo "2. Run: claude 'simulate_input keyboard E tap'"
echo "3. Check Studio output for '[MCPInput] Key: E tap'"
echo ""
