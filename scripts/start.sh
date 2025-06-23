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
         /home/user/data/phoenix

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
envsubst < /home/user/code/eltor-app/backend/bin/torrc.template > /home/user/code/eltor-app/backend/bin/torrc
printenv
envsubst < /home/user/code/eltor-app/backend/bin/torrc.relay.template > /home/user/code/eltor-app/backend/bin/torrc.relay


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
