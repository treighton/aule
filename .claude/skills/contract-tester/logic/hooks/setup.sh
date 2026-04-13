#!/bin/sh
# onInstall hook: install Node.js dependencies for the tool scripts
set -e

cd "$(dirname "$0")/.."
if [ -f package.json ]; then
  echo "Installing dependencies..."
  npm install --production
  echo "Dependencies installed."
else
  echo "No package.json found, skipping npm install."
fi
