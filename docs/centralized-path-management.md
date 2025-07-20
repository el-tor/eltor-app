# Centralized Path Management

This document explains the centralized path management system implemented to address the torrc file and binary path resolution across different platforms (web, Tauri, Docker).

## Problem Statement

Previously, the codebase had scattered path resolution logic across multiple files:

- **Backend** - `routes/eltor.rs` with complex `get_bin_dir()` function
- **Frontend Tauri** - `app_paths.rs` with similar but different logic
- **Docker Scripts** - Hardcoded paths in `scripts/start.sh`
- **Ports Module** - Hardcoded path construction in `ports.rs`
- **Multiple Places** - Various functions constructing paths independently

This led to:
- Duplicate and inconsistent logic
- Platform-specific hardcoded paths 
- Difficulty maintaining across environments
- Template processing scattered in frontend only

## Solution: PathConfig

A new centralized `PathConfig` struct in `backend/src/paths.rs` provides unified path resolution across all platforms.

### Core Features

1. **Environment Detection**: Automatically detects Docker, Tauri, web, and development contexts
2. **Override Support**: Environment variables allow custom deployments
3. **Template Management**: Centralized torrc template processing with variable substitution
4. **Error Handling**: Consistent error messages across platforms
5. **Cross-Platform**: Works on macOS, Linux, Windows with platform-specific executable extensions

### Usage

```rust
use eltor_backend::PathConfig;

// Create path config (auto-detects environment)
let path_config = PathConfig::new()?;

// Get torrc file path
let torrc_path = path_config.get_torrc_path(None); // Default torrc
let torrc_relay_path = path_config.get_torrc_relay_path();

// Get executable paths  
let eltord_binary = path_config.get_executable_path("eltord");
let phoenixd_binary = path_config.get_executable_path("phoenixd");

// Ensure torrc files exist (creates from templates if needed)
path_config.ensure_torrc_files()?;
```

### Environment Variable Overrides

```bash
# Development
export ELTOR_BIN_DIR="$HOME/code/eltor-app/backend/bin"
export ELTOR_DATA_DIR="$HOME/code/eltor-app/backend/bin/data"

# Docker
export ELTOR_DOCKER_ENV=true
export ELTOR_BIN_DIR="/home/user/code/eltor-app/backend/bin"

# Custom deployment  
export ELTOR_BIN_DIR="/opt/eltor/bin"
export ELTOR_DATA_DIR="/var/lib/eltor"
```

### Path Detection Logic

1. **Environment Variable Override**: `ELTOR_BIN_DIR` and `ELTOR_DATA_DIR`
2. **Docker Detection**: Checks for `ELTOR_DOCKER_ENV` or `/home/user/code/eltor-app`
3. **Development Detection**: Uses `CARGO_MANIFEST_DIR` and directory traversal
4. **Tauri Context**: Special handling for app data directory

### Template Processing

The system now handles torrc template substitution centrally:

- Reads from `bin/torrc.template` and `bin/torrc.relay.template`
- Substitutes environment variables (e.g., `$APP_ELTOR_TOR_NICKNAME`)
- Provides sensible defaults for missing environment variables
- Creates files in the appropriate data directory

## Migration Changes

### Backend Changes

1. **New Module**: `backend/src/paths.rs` - Central path configuration
2. **Updated Functions**: All functions using `get_bin_dir()` now use `PathConfig::new()`
3. **Removed Duplication**: Old `get_bin_dir()` marked deprecated, kept for compatibility

### Frontend Changes  

1. **Tauri Integration**: Updated to use backend's `PathConfig` instead of local `app_paths.rs`
2. **Removed Duplication**: No longer maintaining separate path logic
3. **Consistent Behavior**: Same path resolution as backend

### Scripts Changes

Scripts can now use environment variables instead of hardcoded paths:

```bash
# Instead of hardcoded paths
ELTOR_BIN_DIR="${ELTOR_BIN_DIR:-/home/user/code/eltor-app/backend/bin}"
ELTOR_DATA_DIR="${ELTOR_DATA_DIR:-$ELTOR_BIN_DIR/data}"
```

## Files Modified

### Created
- `backend/src/paths.rs` - Central path configuration module

### Modified  
- `backend/src/lib.rs` - Added paths module export, removed get_bin_dir export
- `backend/src/routes/eltor.rs` - Updated to use PathConfig, kept deprecated get_bin_dir
- `backend/src/main.rs` - Updated to use PathConfig for torrc and IP database
- `backend/src/wallet.rs` - Updated phoenixd path resolution 
- `backend/src/ports.rs` - Updated torrc path construction
- `backend/Cargo.toml` - Added rand dependency
- `frontend/src-tauri/src/main.rs` - Removed app_paths, updated to use backend PathConfig

### Deprecated
- `frontend/src-tauri/src/app_paths.rs` - Functions still exist but unused

## Benefits

1. **Single Source of Truth**: All path logic centralized in one module
2. **Environment Detection**: Automatically works across platforms
3. **Override Support**: Environment variables allow custom deployments  
4. **Template Management**: Centralized torrc template processing
5. **Error Handling**: Consistent error messages across platforms
6. **Maintainability**: Easy to update path logic in one place
7. **Testing**: Easier to test path resolution logic in isolation

## Backward Compatibility

- The deprecated `get_bin_dir()` function is kept for compatibility
- Existing code continues to work while migration is gradual
- Environment variable names remain the same for Docker deployments

## Future Improvements

1. **Configuration File**: Could add support for configuration files
2. **Path Validation**: Enhanced validation of paths and permissions
3. **Caching**: Cache path resolution results for performance
4. **Logging**: Enhanced logging of path resolution decisions
