#!/bin/bash
set -euo pipefail

# Mailfeed Update Script
# Updates the binary and restarts the service

INSTALL_DIR="/opt/mailfeed"
SERVICE_NAME="mailfeed"
BACKUP_DIR="/opt/mailfeed-backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "🔄 Updating Mailfeed..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "❌ This script must be run as root (use sudo)"
   exit 1
fi

# Check if new binary exists
if [[ ! -f "./target/release/mailfeed" ]]; then
    echo "❌ New binary not found. Please run 'cargo build --release' first."
    exit 1
fi

# Create backup before update
echo "📦 Creating backup before update..."
mkdir -p "$BACKUP_DIR"
systemctl stop "$SERVICE_NAME"

tar -czf "$BACKUP_DIR/pre_update_backup_$TIMESTAMP.tar.gz" \
    -C "$INSTALL_DIR" \
    bin/mailfeed \
    data/mailfeed.db \
    config/environment \
    || echo "⚠️  Backup created with warnings"

# Update binary
echo "📦 Installing new binary..."
cp ./target/release/mailfeed "$INSTALL_DIR/bin/mailfeed.new"
chmod +x "$INSTALL_DIR/bin/mailfeed.new"
chown mailfeed:mailfeed "$INSTALL_DIR/bin/mailfeed.new"

# Atomic replace
mv "$INSTALL_DIR/bin/mailfeed.new" "$INSTALL_DIR/bin/mailfeed"

# Update static files if available
if [[ -d "./static" ]]; then
    echo "🎨 Updating static files..."
    rm -rf "$INSTALL_DIR/public.new"
    mkdir -p "$INSTALL_DIR/public.new"
    cp -r ./static/* "$INSTALL_DIR/public.new/"
    chown -R mailfeed:mailfeed "$INSTALL_DIR/public.new"
    
    # Atomic replace
    if [[ -d "$INSTALL_DIR/public" ]]; then
        mv "$INSTALL_DIR/public" "$INSTALL_DIR/public.old"
    fi
    mv "$INSTALL_DIR/public.new" "$INSTALL_DIR/public"
    rm -rf "$INSTALL_DIR/public.old"
fi

# Restart service
echo "🔄 Restarting service..."
systemctl start "$SERVICE_NAME"

# Wait a moment for startup
sleep 3

# Check service status
if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo "✅ Update completed successfully!"
    echo ""
    echo "📊 Service status:"
    systemctl status "$SERVICE_NAME" --no-pager -l
    echo ""
    echo "🔗 Health check:"
    curl -s http://localhost:8080/health | jq . || echo "Health check endpoint not responding"
else
    echo "❌ Service failed to start! Rolling back..."
    
    # Rollback
    systemctl stop "$SERVICE_NAME" || true
    tar -xzf "$BACKUP_DIR/pre_update_backup_$TIMESTAMP.tar.gz" -C "$INSTALL_DIR"
    chown -R mailfeed:mailfeed "$INSTALL_DIR"
    systemctl start "$SERVICE_NAME"
    
    echo "🔙 Rollback completed. Check logs: sudo journalctl -u mailfeed -n 50"
    exit 1
fi

echo ""
echo "📋 Next steps:"
echo "   - Check logs: sudo journalctl -u mailfeed -f"
echo "   - Test application: http://your-server:8080"
echo "   - Backup location: $BACKUP_DIR/pre_update_backup_$TIMESTAMP.tar.gz"