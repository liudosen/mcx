#!/bin/bash
set -e

PID=$(lsof -ti :8081 2>/dev/null || true)
if [ -n "$PID" ]; then
    echo "Killing process $PID on port 8081..."
    kill -9 $PID
    sleep 1
fi

echo "Starting backend..."
cargo run
