#!/bin/bash
set -e

echo "🚀 Starting Eltor Application..."
echo "🔧 Backend will run on http://0.0.0.0:8080"
echo "🌐 Frontend will be served from http://0.0.0.0:5173"
echo ""

# include environment variables
. /home/eltor/exports.sh

# run phoenixd 
# TODO check if USE_PHOENIXD_EMBEDDED=true
cd /home/eltor/code/eltor-app/backend/bin
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
envsubst < /home/eltor/code/eltor-app/backend/bin/torrc.template > /home/eltor/code/eltor-app/backend/bin/torrc
printenv
envsubst < /home/eltor/code/eltor-app/backend/bin/torrc.relay.template > /home/eltor/code/eltor-app/backend/bin/torrc.relay

mkdir -p /home/eltor/data/tor/client
mkdir -p /home/eltor/data/tor-relay/client

# Start backend in background
cd /home/eltor/code/eltor-app/backend/bin
echo "📡 Starting backend server..."
./eltor-backend &
BACKEND_PID=$!

# Start frontend
cd /home/eltor/code/eltor-app/frontend/dist
echo "🌐 Starting frontend server..."
python3 -m http.server 5173 --bind 0.0.0.0 &
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
