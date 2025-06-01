# El Tor Backend

Rust backend server for the El Tor application.

## Features

- **Tor Control**: Connect/disconnect from Tor network
- **Eltord Management**: Start/stop eltord processes
- **CORS Enabled**: Works with frontend on localhost:5173
- **Process Tracking**: Tracks running eltord processes

## API Endpoints

### Health
- `GET /health` - Check if server is running

### Tor Control
- `POST /api/tor/connect` - Connect to Tor
- `POST /api/tor/disconnect` - Disconnect from Tor  
- `GET /api/tor/status` - Get Tor connection status

### Eltord Management
- `POST /api/eltord/activate` - Start eltord process
- `POST /api/eltord/deactivate` - Stop eltord process
- `GET /api/eltord/status` - Get eltord process status

## Prerequisites

- Rust 1.70+
- eltord project at `~/code/eltord/`

## Running

```bash
cargo run
# or
./run.sh

## Bin
The bin directory contains symbolic links of the 
- eltord binary
- torrc config file
- phoenixd binary

To link
```
ln -s ~/code/eltord/target/debug/eltor ~/code/eltor-app/backend/bin/eltord
ln -s ~/code/eltord/torrc.client.prod ~/code/eltor-app/backend/bin/torrc
# manually copied phoenixd and phoenix-cli to the bin
```