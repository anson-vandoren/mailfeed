#!/bin/bash
set -euo pipefail

# Mailfeed Maintenance Script
# Performs routine maintenance tasks

INSTALL_DIR="/opt/mailfeed"
SERVICE_NAME="mailfeed"
DB_PATH="$INSTALL_DIR/data/mailfeed.db"
BACKUP_DIR="/opt/mailfeed-backups"

log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1"
}

cleanup_old_sessions() {
    log_message "🧹 Cleaning up expired sessions..."
    
    # Use sqlite3 to clean up sessions older than 7 days
    sqlite3 "$DB_PATH" "DELETE FROM sessions WHERE created_at < datetime('now', '-7 days');" 2>/dev/null || {
        log_message "⚠️  Session cleanup failed - database may be in use"
        return 1
    }
    
    local deleted=$(sqlite3 "$DB_PATH" "SELECT changes();" 2>/dev/null || echo "0")
    log_message "📊 Cleaned up $deleted expired sessions"
}

cleanup_old_feed_items() {
    log_message "🗄️  Cleaning up old feed items..."
    
    # Keep feed items for 30 days
    sqlite3 "$DB_PATH" "DELETE FROM feed_items WHERE created_at < datetime('now', '-30 days');" 2>/dev/null || {
        log_message "⚠️  Feed item cleanup failed - database may be in use"
        return 1
    }
    
    local deleted=$(sqlite3 "$DB_PATH" "SELECT changes();" 2>/dev/null || echo "0")
    log_message "📊 Cleaned up $deleted old feed items"
}

vacuum_database() {
    log_message "🗜️  Optimizing database..."
    
    # Stop service temporarily for vacuum
    systemctl stop "$SERVICE_NAME"
    
    # Vacuum database
    sqlite3 "$DB_PATH" "VACUUM;" 2>/dev/null || {
        log_message "❌ Database vacuum failed"
        systemctl start "$SERVICE_NAME"
        return 1
    }
    
    # Restart service
    systemctl start "$SERVICE_NAME"
    
    log_message "✅ Database optimized"
}

cleanup_old_backups() {
    log_message "🗑️  Cleaning up old backups..."
    
    if [[ -d "$BACKUP_DIR" ]]; then
        # Keep backups for 30 days
        local deleted=0
        find "$BACKUP_DIR" -name "*.tar.gz" -mtime +30 -delete 2>/dev/null || true
        
        # Count remaining backups
        local remaining=$(ls -1 "$BACKUP_DIR"/*.tar.gz 2>/dev/null | wc -l || echo "0")
        log_message "📊 $remaining backups remaining"
    fi
}

cleanup_logs() {
    log_message "📝 Cleaning up system logs..."
    
    # Clean journalctl logs older than 30 days
    journalctl --vacuum-time=30d >/dev/null 2>&1 || true
    
    # Clean application logs
    if [[ -d "$INSTALL_DIR/logs" ]]; then
        find "$INSTALL_DIR/logs" -name "*.log" -mtime +30 -delete 2>/dev/null || true
    fi
    
    log_message "✅ Log cleanup completed"
}

check_disk_usage() {
    log_message "💽 Checking disk usage..."
    
    local usage_percent=$(df "$INSTALL_DIR" | awk 'NR==2 {print $5}' | sed 's/%//')
    local available_gb=$(df -h "$INSTALL_DIR" | awk 'NR==2 {print $4}')
    
    log_message "📊 Disk usage: ${usage_percent}% (${available_gb} available)"
    
    if [[ "$usage_percent" -gt 80 ]]; then
        log_message "⚠️  Warning: Disk usage is high (>${usage_percent}%)"
        return 1
    fi
}

generate_report() {
    log_message "📊 Generating maintenance report..."
    
    # Service status
    local service_status="unknown"
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        service_status="running"
    else
        service_status="stopped"
    fi
    
    # Database size
    local db_size="0"
    if [[ -f "$DB_PATH" ]]; then
        db_size=$(du -h "$DB_PATH" | cut -f1)
    fi
    
    # Memory usage
    local memory_usage=$(ps -o rss= -p $(pgrep -f mailfeed) 2>/dev/null | awk '{sum+=$1} END {print sum/1024}' || echo "0")
    
    # Uptime
    local uptime=$(systemctl show "$SERVICE_NAME" --property=ActiveEnterTimestamp --value 2>/dev/null | head -1)
    
    cat << EOF

📊 Maintenance Report - $(date)
============================================
🔧 Service Status: $service_status
💾 Database Size: $db_size
🧠 Memory Usage: ${memory_usage} MB
⏱️  Service Uptime: $uptime
💽 Disk Usage: $(df -h "$INSTALL_DIR" | awk 'NR==2 {print $5 " used, " $4 " available"}')
📦 Backup Count: $(ls -1 "$BACKUP_DIR"/*.tar.gz 2>/dev/null | wc -l || echo "0")

EOF
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Maintenance options:
  --sessions      Clean up expired sessions only
  --feed-items    Clean up old feed items only  
  --database      Vacuum/optimize database only
  --backups       Clean up old backups only
  --logs          Clean up old logs only
  --disk-check    Check disk usage only
  --report        Generate maintenance report only
  --all           Run full maintenance (default)
  --help          Show this help message

Examples:
  $0                    # Run full maintenance
  $0 --sessions         # Clean up sessions only
  $0 --report           # Generate report only
EOF
}

main() {
    local run_all=true
    local run_sessions=false
    local run_feed_items=false
    local run_database=false
    local run_backups=false
    local run_logs=false
    local run_disk_check=false
    local run_report=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --sessions)
                run_all=false
                run_sessions=true
                shift
                ;;
            --feed-items)
                run_all=false
                run_feed_items=true
                shift
                ;;
            --database)
                run_all=false
                run_database=true
                shift
                ;;
            --backups)
                run_all=false
                run_backups=true
                shift
                ;;
            --logs)
                run_all=false
                run_logs=true
                shift
                ;;
            --disk-check)
                run_all=false
                run_disk_check=true
                shift
                ;;
            --report)
                run_all=false
                run_report=true
                shift
                ;;
            --all)
                run_all=true
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    log_message "🔧 Starting maintenance tasks..."
    
    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        log_message "❌ This script must be run as root (use sudo)"
        exit 1
    fi
    
    # Run selected tasks
    if [[ "$run_all" == "true" ]] || [[ "$run_sessions" == "true" ]]; then
        cleanup_old_sessions
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_feed_items" == "true" ]]; then
        cleanup_old_feed_items
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_database" == "true" ]]; then
        vacuum_database
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_backups" == "true" ]]; then
        cleanup_old_backups
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_logs" == "true" ]]; then
        cleanup_logs
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_disk_check" == "true" ]]; then
        check_disk_usage
    fi
    
    if [[ "$run_all" == "true" ]] || [[ "$run_report" == "true" ]]; then
        generate_report
    fi
    
    log_message "✅ Maintenance completed"
}

# Run maintenance
main "$@"