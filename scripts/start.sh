#!/bin/bash
set -e

# Set default environment variables if not provided
export BACKEND_PORT=${BACKEND_PORT:-${PORT:-5174}}
export BIND_ADDRESS=${BIND_ADDRESS:-0.0.0.0}  # Default to 0.0.0.0 for Docker
BACKEND_URL=${BACKEND_URL:-http://localhost:${BACKEND_PORT}}

echo "🚀 Starting Eltor Application..."
echo "🔧 Backend will run on http://${BIND_ADDRESS}:$BACKEND_PORT"
echo "🌐 Frontend will be served from the backend (integrated mode)"
echo ""

printenv

# Ensure data directories exist with proper permissions
echo "📁 Creating data directories..."
mkdir -p $APP_ELTOR_TOR_DATA_DIRECTORY/logs \
         $APP_ELTOR_TOR_DATA_DIRECTORY/client \
         $APP_ELTOR_TOR_RELAY_DATA_DIRECTORY/client \
         $APP_ELTOR_USER_DIR/data/phoenix \
         $APP_ELTOR_USER_DIR/code/eltor-app/backend/bin/data

# include environment variables
. $APP_ELTOR_USER_DIR/exports.sh

##############################################
# Phoenixd 
##############################################
if [ "$APP_ELTOR_USE_PHOENIXD_EMBEDDED" = "true" ]; then
    echo "🔧 Embedded Phoenix mode enabled, starting Phoenix daemon..."
    cd /home/user/code/eltor-app/backend/bin
    ./phoenixd &
    PHOENIX_PID=$!
    # Wait for Phoenix daemon to start
    echo "⏳ Waiting for Phoenix daemon to start..."
    sleep 5

    # Parse phoenixd password from the conf and copy to torrc
    get_phoenixd_password() {
        awk -F'=' '/^http-password=/ {print $2}' ~/.phoenix/phoenix.conf
    }
    PHOENIXD_PASSWORD=$(get_phoenixd_password)
    export APP_ELTOR_LN_CONFIG="type=phoenixd url=http://127.0.0.1:9740 password=$PHOENIXD_PASSWORD default=true"
    export APP_ELTOR_LN_IMPLEMENTATION="phoenixd"    
    # Get BOLT12 offer from Phoenix daemon
    echo "🔍 Fetching BOLT12 offer from Phoenix daemon..."
    BOLT12_OFFER=""
    
    # Try to get the offer from Phoenix API
    for attempt in 1 2 3; do
        echo "⏳ Attempt $attempt to get BOLT12 offer..."
        BOLT12_OFFER=$(curl -s -u ":$PHOENIXD_PASSWORD" \
            -X GET "http://127.0.0.1:9740/getoffer" \
            -H "Content-Type: application/x-www-form-urlencoded" \
            -d "description=Eltor%20Relay%20Payment" 2>/dev/null || echo "")
        
        if [ -n "$BOLT12_OFFER" ] && [ "$BOLT12_OFFER" != "null" ] && [[ "$BOLT12_OFFER" == lno* ]]; then
            echo "✅ Got BOLT12 offer: ${BOLT12_OFFER:0:50}..."
            break
        else
            echo "⚠️  Failed to get BOLT12 offer, retrying in 2 seconds..."
            sleep 2
        fi
    done
    
    # Set the offer or use a placeholder
    if [ -n "$BOLT12_OFFER" ] && [ "$BOLT12_OFFER" != "null" ]; then
        export APP_ELTOR_LN_BOLT12="$BOLT12_OFFER"
    else
        echo "❌ Could not retrieve BOLT12 offer from Phoenix, using placeholder"
        export APP_ELTOR_LN_BOLT12="lno-phoenix-offer-unavailable"
    fi
    
    echo "✅ Phoenix daemon configured with password from ~/.phoenix/phoenix.conf"
else
    echo "🔧 Using external Lightning implementation: $APP_ELTOR_LN_IMPLEMENTATION"
    echo "📡 Lightning config: $APP_ELTOR_LN_CONFIG"
fi


##############################################
# torrc 
##############################################
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


##############################################
# IP address 
##############################################
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



##############################################
# Start App 
##############################################
# With the new integrated backend, we only need to start the backend
# which will serve both the API and the frontend static files
echo "� Starting integrated Eltor server..."
echo "   Backend API: http://$BIND_ADDRESS:$BACKEND_PORT/api/*"
echo "   Frontend: http://$BIND_ADDRESS:$BACKEND_PORT/"
echo "   Environment variables will be injected into frontend automatically"
cd /home/user/code/eltor-app
./backend/bin/eltor-backend &
SERVER_PID=$!

##############################################
# Cleanup on exit
##############################################
# Function to cleanup on exit
cleanup() {
    echo "🛑 Shutting down server..."
    kill $SERVER_PID 2>/dev/null || true
    if [ "$APP_ELTOR_USE_PHOENIXD_EMBEDDED" = "true" ] && [ -n "$PHOENIX_PID" ]; then
        echo "🛑 Shutting down Phoenix daemon..."
        kill $PHOENIX_PID 2>/dev/null || true
    fi
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Wait for the server process
wait $SERVER_PID
