#!/bin/bash

# Stop consensus validator network
set -euo pipefail

NETWORK_NAME="otter"

echo "=== Stopping OtterEVM Network ==="

# Stop and remove all validator containers
for i in {0..3}; do
  container_name="otter-validator-$i"
  if docker ps -a -q -f name="$container_name" | grep -q .; then
    echo "Stopping $container_name..."
    docker stop "$container_name" >/dev/null 2>&1 || true
    docker rm "$container_name" >/dev/null 2>&1 || true
    echo "  ✓ Stopped $container_name"
  else
    echo "  - $container_name not found"
  fi
done

# Remove Docker network
if docker network ls | grep -q "$NETWORK_NAME"; then
  echo "Removing Docker network: $NETWORK_NAME"
  docker network rm "$NETWORK_NAME" >/dev/null
  echo "  ✓ Removed network"
fi

echo "=== Network Stopped ==="

