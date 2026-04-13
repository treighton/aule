#!/bin/sh
# onActivate hook: verify Node.js is available and meets version constraint
set -e

if ! command -v node >/dev/null 2>&1; then
  echo "ERROR: Node.js is not installed. This skill requires Node.js >= 18." >&2
  exit 1
fi

NODE_VERSION=$(node --version | sed 's/^v//')
NODE_MAJOR=$(echo "$NODE_VERSION" | cut -d. -f1)

if [ "$NODE_MAJOR" -lt 18 ]; then
  echo "ERROR: Node.js >= 18 required, found v${NODE_VERSION}" >&2
  exit 1
fi

echo "Node.js v${NODE_VERSION} verified."
