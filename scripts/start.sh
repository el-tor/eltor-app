#!/bin/bash
set -e

echo "🚀 Starting Eltor Application..."
echo "🔧 Backend will run on http://0.0.0.0:8080"
echo "🌐 Frontend will be served from http://0.0.0.0:5173"
echo ""

# include environment variables
. /app/exports.sh

# run phoenixd 
# TODO check if USE_PHOENIXD_EMBEDDED=true
cd /root/code/eltor-app/backend/bin
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
envsubst < /root/code/eltor-app/backend/bin/torrc.template > /root/code/eltor-app/backend/bin/torrc
printenv
envsubst < /root/code/eltor-app/backend/bin/torrc.relay.template > /root/code/eltor-app/backend/bin/torrc.relay

mkdir -p /app/data/tor/client
mkdir -p /app/data/tor-relay/client

# Start backend in background
cd /root/code/eltor-app/backend/bin
echo "📡 Starting backend server..."
./eltor-backend &
BACKEND_PID=$!

# Start frontend with Vite preview server
cd /root/code/eltor-app/frontend
echo "🌐 Starting frontend server with Vite preview..."
npx vite preview --port 5173 --host 0.0.0.0 --no-open &
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
