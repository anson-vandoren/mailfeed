#!/bin/bash
set -euo pipefail

# Simple backup script for Mailfeed
INSTALL_DIR="/opt/mailfeed"
BACKUP_DIR="/opt/mailfeed-backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="mailfeed_backup_$TIMESTAMP"

echo "📦 Creating backup: $BACKUP_NAME"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Stop service temporarily
echo "⏸️  Stopping mailfeed service..."
systemctl stop mailfeed

# Create backup
echo "💾 Backing up database and configuration..."
tar -czf "$BACKUP_DIR/$BACKUP_NAME.tar.gz" \
    -C "$INSTALL_DIR" \
    data/mailfeed.db \
    config/environment \
    || echo "⚠️  Backup created with warnings"

# Start service
echo "▶️  Starting mailfeed service..."
systemctl start mailfeed

# Cleanup old backups (keep last 7 days)
echo "🧹 Cleaning up old backups..."
find "$BACKUP_DIR" -name "mailfeed_backup_*.tar.gz" -mtime +7 -delete

echo "✅ Backup complete: $BACKUP_DIR/$BACKUP_NAME.tar.gz"
echo "📊 Backup size: $(du -h "$BACKUP_DIR/$BACKUP_NAME.tar.gz" | cut -f1)"
echo "📁 Total backups: $(ls -1 "$BACKUP_DIR"/mailfeed_backup_*.tar.gz 2>/dev/null | wc -l)"