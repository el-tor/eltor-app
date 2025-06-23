#!/bin/bash
set -e

# Set default environment variables if not provided
export BACKEND_PORT=${BACKEND_PORT:-5174}
export FRONTEND_PORT=${FRONTEND_PORT:-5173}

echo "🚀 Starting Eltor Application..."
echo "🔧 Backend will run on http://0.0.0.0:$BACKEND_PORT"
echo "🌐 Frontend will be served from http://0.0.0.0:$FRONTEND_PORT"
echo ""

# Ensure data directories exist with proper permissions
echo "📁 Creating data directories..."
mkdir -p /home/user/data/logs \
         /home/user/data/tor/client \
         /home/user/data/tor-relay/client \
         /home/user/data/phoenix \
         /home/user/code/eltor-app/backend/bin/data

# include environment variables
. /home/user/exports.sh

# run phoenixd 
# TODO check if USE_PHOENIXD_EMBEDDED=true
cd /home/user/code/eltor-app/backend/bin
echo "🔧 Starting Phoenix daemon..." 
./phoenixd &
PHOENIX_PID=$!
# Wait for Phoenix daemon to start
echo "⏳ Waiting for Phoenix daemon to start..."
sleep 5
kill $PHOENIX_PID 2>/dev/null || true

# Parse phoenixd password from the conf and copy to torrc
get_phoenixd_password() {
    awk -F'=' '/^http-password=/ {print $2}' ~/.phoenix/phoenix.conf
}
PHOENIXD_PASSWORD=$(get_phoenixd_password)
export TOR_PAYMENT_LIGHTNING_NODE_CONFIG="type=phoenixd url=http://127.0.0.1:9740 password=$PHOENIXD_PASSWORD default=true"

# Generate torrc files only if they don't exist
echo "📝 Ensuring torrc data directory exists and is writable..."
mkdir -p /home/user/code/eltor-app/backend/bin/data
chmod 755 /home/user/code/eltor-app/backend/bin/data

# Remove any broken symlinks in the data directory
echo "🧹 Cleaning up any broken symlinks..."
find /home/user/code/eltor-app/backend/bin/data -type l -exec test ! -e {} \; -delete

# Debug: Check directory and permissions
echo "🔍 Debug: Checking directory structure..."
ls -la /home/user/code/eltor-app/backend/bin/ || echo "❌ backend/bin directory not found"
ls -la /home/user/code/eltor-app/backend/bin/data/ || echo "❌ backend/bin/data directory not found"
pwd
whoami
id

if [ ! -f "/home/user/code/eltor-app/backend/bin/data/torrc" ]; then
    echo "📝 Generating torrc configuration..."
    if [ -f "/home/user/code/eltor-app/backend/bin/torrc.template" ]; then
        echo "✅ Template found, creating torrc..."
        # Remove any existing torrc file or symlink
        rm -f /home/user/code/eltor-app/backend/bin/data/torrc
        # Test if we can write to the directory first
        touch /home/user/code/eltor-app/backend/bin/data/test.tmp && rm /home/user/code/eltor-app/backend/bin/data/test.tmp
        if [ $? -eq 0 ]; then
            envsubst < /home/user/code/eltor-app/backend/bin/torrc.template > /home/user/code/eltor-app/backend/bin/data/torrc
            echo "✅ torrc generated successfully"
        else
            echo "❌ Cannot write to /home/user/code/eltor-app/backend/bin/data/ directory"
            exit 1
        fi
    else
        echo "❌ torrc.template not found at /home/user/code/eltor-app/backend/bin/torrc.template"
        ls -la /home/user/code/eltor-app/backend/bin/ || echo "❌ Cannot list backend/bin directory"
        exit 1
    fi
else
    echo "✅ torrc already exists, skipping generation"
fi

if [ ! -f "/home/user/code/eltor-app/backend/bin/data/torrc.relay" ]; then
    echo "📝 Generating torrc.relay configuration..."
    if [ -f "/home/user/code/eltor-app/backend/bin/torrc.relay.template" ]; then
        echo "✅ Template found, creating torrc.relay..."
        # Remove any existing torrc.relay file or symlink
        rm -f /home/user/code/eltor-app/backend/bin/data/torrc.relay
        envsubst < /home/user/code/eltor-app/backend/bin/torrc.relay.template > /home/user/code/eltor-app/backend/bin/data/torrc.relay
        echo "✅ torrc.relay generated successfully"
    else
        echo "❌ torrc.relay.template not found at /home/user/code/eltor-app/backend/bin/torrc.relay.template"
        ls -la /home/user/code/eltor-app/backend/bin/ || echo "❌ Cannot list backend/bin directory"
        exit 1
    fi
else
    echo "✅ torrc.relay already exists, skipping generation"
fi

# Function to get public IP address
get_public_ip() {
    local ip=""
    
    # Try multiple services to get public IP
    for service in "ifconfig.me" "ipinfo.io/ip" "icanhazip.com" "checkip.amazonaws.com"; do
        echo "🌐 Trying to get public IP from $service..." >&2
        ip=$(curl -s --connect-timeout 5 --max-time 10 "http://$service" 2>/dev/null | tr -d '\n\r' | grep -E '^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$')
        
        if [ -n "$ip" ]; then
            echo "✅ Got public IP: $ip" >&2
            echo "$ip"
            return 0
        fi
    done
    
    echo "❌ Failed to get public IP from all services" >&2
    return 1
}

# Function to update Address line in torrc.relay
update_torrc_relay_address() {
    local torrc_file="/home/user/code/eltor-app/backend/bin/data/torrc.relay"
    local public_ip
    
    if [ ! -f "$torrc_file" ]; then
        echo "⚠️  torrc.relay file not found at $torrc_file"
        return 1
    fi
    
    public_ip=$(get_public_ip)
    if [ $? -ne 0 ] || [ -z "$public_ip" ]; then
        echo "⚠️  Could not get public IP, skipping Address update in torrc.relay"
        return 1
    fi
    
    # Check if Address line exists
    if grep -q "^Address " "$torrc_file"; then
        echo "📝 Updating existing Address line in torrc.relay with IP: $public_ip"
        sed -i.bak "s/^Address .*/Address $public_ip/" "$torrc_file"
    else
        echo "📝 Adding Address line to torrc.relay with IP: $public_ip"
        echo "Address $public_ip" >> "$torrc_file"
    fi
    
    echo "✅ torrc.relay Address updated successfully"
}

# Update the Address in torrc.relay with current public IP
update_torrc_relay_address

printenv


# Start backend in background
cd /home/user/code/eltor-app/backend/bin
echo "📡 Starting backend server..."
./eltor-backend &
BACKEND_PID=$!

# Start frontend
cd /home/user/code/eltor-app/frontend/dist
echo "🌐 Starting frontend server..."
python3 -m http.server $FRONTEND_PORT --bind 0.0.0.0 &
FRONTEND_PID=$!

# Function to cleanup on exit
cleanup() {
    echo "🛑 Shutting down services..."
    kill $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Wait for both processes
wait $BACKEND_PID $FRONTEND_PID
