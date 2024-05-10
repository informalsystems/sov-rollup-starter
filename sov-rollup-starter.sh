#!/usr/bin/env bash

function run_make_cmd() {
    local cmd="$1"
    echo "Running: '$cmd'"
    $cmd
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "Expected exit code 0, got $exit_code"
        exit 1
    fi
}

trap 'jobs -p | xargs -r kill' EXIT
echo 'Running: '\''cd crates/rollup/'\'''
cd crates/rollup/
if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo 'Running: '\''make clean-db'\'''
make clean-db
if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo 'Running: '\''cargo run --bin node'\'''
output=$(mktemp)
cargo run --bin node &> $output &
background_process_pid=$!
echo "Waiting for process with PID: $background_process_pid"
until grep -q -i RPC $output
do
  if ! ps $background_process_pid > /dev/null
  then
    echo "The background process died" >&2
    exit 1
  fi
  echo -n "."
  sleep 5
done

echo 'Running: '\''make test-create-token'\'''
make test-create-token
if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo 'Running: '\''make wait-ten-seconds'\'''
make wait-ten-seconds
if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo 'Running: '\''make test-bank-supply-of'\'''
make test-bank-supply-of
if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo 'Running: '\''curl -X POST -H "Content-Type: application/json" -d '\''{"jsonrpc":"2.0","method":"bank_supplyOf","params":{"token_id":"token_1gs3c7xshs42cr09d5yj0np6smhnd6y0fyleas8qpvk9u020crj2q5ar73r"},"id":1}'\'' http://127.0.0.1:12345'\'''
output=$(curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"bank_supplyOf","params":{"token_id":"token_1gs3c7xshs42cr09d5yj0np6smhnd6y0fyleas8qpvk9u020crj2q5ar73r"},"id":1}' http://127.0.0.1:12345)
expected='{"jsonrpc":"2.0","result":{"amount":103000000},"id":1}
'
# Either of the two must be a substring of the other. This kinda protects us
# against whitespace differences, trimming, etc.
if ! [[ $output == *"$expected"* || $expected == *"$output"* ]]; then
    echo "'$expected' not found in text:"
    echo "'$output'"
    exit 1
fi

run_make_cmd "make test-create-client"
run_make_cmd "make wait-ten-seconds"
run_make_cmd "make test-query-client-state"
run_make_cmd "make test-update-client"
run_make_cmd "make wait-ten-seconds"
run_make_cmd "make test-query-client-status"

if [ $? -ne 0 ]; then
    echo "Expected exit code 0, got $?"
    exit 1
fi

echo "All tests passed!"; exit 0
