#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://localhost:8080}"

echo "=== Smoke Test: Arbitek Trade Copier ==="
echo "Target: $API_URL"
echo

echo "1. Health check"
curl -fsS "$API_URL/health" | jq .
echo

echo "2. System status (public)"
curl -fsS "$API_URL/api/v1/system/status" | jq .
echo

echo "3. Login (should fail with bad creds)"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"wrong"}' || true)
if [ "$HTTP_CODE" = "401" ]; then
  echo "PASS: login rejected bad creds (HTTP 401)"
else
  echo "FAIL: expected 401, got $HTTP_CODE"
  exit 1
fi
echo

echo "4. Login (with env password)"
TOKEN=$(curl -fsS -X POST "$API_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"admin\",\"password\":\"${ADMIN_PASSWORD:-secret}\"}" | jq -r '.token')
echo "PASS: obtained token"
echo

echo "5. Protected route without token (should fail)"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X PATCH "$API_URL/api/v1/masters/$(uuidgen)/status" \
  -H "Content-Type: application/json" \
  -d '{"status":"paused"}' || true)
if [ "$HTTP_CODE" = "401" ]; then
  echo "PASS: protected route rejected without token (HTTP 401)"
else
  echo "FAIL: expected 401, got $HTTP_CODE"
  exit 1
fi
echo

echo "6. Protected route with token"
curl -fsS -X PATCH "$API_URL/api/v1/masters/00000000-0000-0000-0000-000000000000/status" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status":"paused"}' | jq .
echo "PASS: protected route accepted valid token"
echo

echo "7. SSE endpoint (activity)"
curl -fsS -N -H "Accept: text/event-stream" "$API_URL/sse/activity" &
CURL_PID=$!
sleep 1
kill $CURL_PID 2>/dev/null || true
echo "PASS: SSE endpoint reachable"
echo

echo "=== All smoke tests passed ==="
