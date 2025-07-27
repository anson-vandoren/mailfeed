# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Mailfeed is a self-hosted RSS/Atom feed-to-email service built with Rust (backend) and HTMX + Askama templates (frontend). It allows users to subscribe to feeds and receive them as emails on configurable schedules (realtime, hourly, daily).

## Development Commands

### Backend (Rust)
```bash
# Navigate to backend directory
cd mailfeed

# Run in development mode  
cargo run

# Run all tests
cargo test

# Run specific test categories
cargo test --test integration_tests
cargo test --test auth_tests  
cargo test --test e2e_tests

# Run unit tests only
cargo test --lib

# Create admin user (interactive CLI)
cargo run --release -- --create-admin

# Install diesel CLI for database migrations
cargo install diesel_cli --no-default-features --features "sqlite"

# Database operations
diesel migration run    # Apply pending migrations
diesel migration revert # Revert last migration
```

### Frontend (HTMX + Askama)
The frontend is now server-side rendered using Askama templates with HTMX for interactivity. Templates are located in the `templates/` directory and static assets in `static/`.

No build step required - templates are compiled at runtime by the Rust backend.

## Architecture Overview

### Backend Structure (Rust + Actix Web)
- **Entry Point**: `src/main.rs` - Server setup, CLI commands, database initialization
- **API Layer**: `src/api/` - RESTful endpoints organized by resource
  - `auth/` - Session-based authentication (login/logout)
  - `users/` - User management (CRUD operations)
  - `feeds/` - Feed management (admin only)
  - `subscriptions/` - User subscription management
  - `feed_items/` - Feed item access (admin only)
- **Models**: `src/models/` - Database models using Diesel ORM
- **Database**: `src/schema.rs` - Auto-generated Diesel schema
- **Background Tasks**: `src/tasks/` - Async workers
  - `feed_monitor/` - Polls feeds for updates (~5min intervals)  
  - `email_sender/` - Sends scheduled emails based on subscription frequency
- **Auth**: Session-based authentication with cookies
- **Web UI**: `src/web_ui.rs` - HTMX endpoints serving Askama templates

### Frontend Structure (HTMX + Askama)
- **Templates**: `templates/` - Server-side rendered HTML with Askama
- **Static Assets**: `static/` - Favicon, fonts, and other static files
- **Authentication**: Session cookies managed by server
- **UI Styling**: Skeleton UI classes embedded in base template
- **Interactivity**: HTMX for dynamic content updates

### Key Data Flow
1. Users authenticate → Session cookies set by server
2. Feed subscriptions created → stored in database with user association
3. Background `feed_monitor` polls RSS/Atom feeds → updates `feed_items` table
4. Background `email_sender` checks schedules → sends emails based on subscription frequency
5. Frontend communicates with backend via HTMX requests and REST API with session authentication

### Database Schema (SQLite + Diesel)
- `users` - User accounts with roles (admin/user), email settings
- `feeds` - RSS/Atom feed metadata and polling status
- `feed_items` - Individual posts/articles from feeds
- `subscriptions` - User-to-feed relationships with scheduling preferences
- `settings` - Key-value configuration storage

## Configuration

### Environment Variables
- `MF_DATABASE_URL` - SQLite database path (default: `./mailfeed.db`)
- `MF_PUBLIC_PATH` - Static file serving directory (default: `./static`)
- `MF_PORT` - Server port (default: 8080)

### Development Setup Requirements
- Rust toolchain + Cargo
- Node.js + npm
- SQLite development libraries: `sudo apt install libsqlite3-dev`
- Diesel CLI for database migrations

## API Design Patterns
- RESTful endpoints with resource-based organization
- Session-based authentication with secure cookies
- Admin vs user role-based access control
- Database connection pooling (r2d2)
- CORS enabled for cross-origin requests
- JSON request/response format
- Structured error handling with appropriate HTTP status codes

## Authentication Flow
1. POST `/api/auth/login` → sets secure session cookie
2. Session cookie automatically included in subsequent requests
3. POST `/api/auth/logout` to clear session cookie and invalidate server-side session

# Dev Actions

**Important: do not ever touch a file outside of the directory that this CLAUDE.md is in**

**Deployment Note**: This app is designed for small, self-hosted deployments as a standalone Rust binary. No containerization (Docker) is needed or desired - the compiled binary can be deployed directly to target systems.

