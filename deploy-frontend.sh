#!/bin/bash
set -e

FRONTEND_DIR="/root/workspace/welfare-store/frontend"
DIST_DIR="$FRONTEND_DIR/dist"
NGINX_DIR="/usr/share/nginx/html"

echo "=== Frontend Deployment Script ==="

# Check if dist exists
if [ ! -d "$DIST_DIR" ]; then
    echo "Error: dist directory not found. Building frontend first..."
    cd "$FRONTEND_DIR"
    npm run build:prod
fi

# Sync to nginx directory
echo "Syncing dist to nginx directory..."
rsync -av --delete "$DIST_DIR/" "$NGINX_DIR/"

echo "Done! Frontend deployed to $NGINX_DIR"
