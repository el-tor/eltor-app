
######################################
### App env vars
######################################
export APP_ELTOR_FRONTEND_PORT="5173"
export APP_ELTOR_BACKEND_PORT="5174"
export APP_ELTOR_HOST="eltor-app_web_1"
export APP_ELTOR_USER_DIR="/home/user"
export APP_ELTOR_ELTORRC_PATH="$APP_ELTOR_USER_DIR/code/eltor-app/backend/bin/data"



######################################
### LN env vars
######################################
export APP_ELTOR_USE_PHOENIXD_EMBEDDED="$APP_ELTOR_USE_PHOENIXD_EMBEDDED"
export APP_ELTOR_LN_IMPLEMENTATION="$APP_ELTOR_LN_IMPLEMENTATION"
export APP_ELTOR_LN_CONFIG=$APP_ELTOR_LN_CONFIG

######################################
### Client Eltor Environment Variables
######################################
export APP_ELTOR_TOR_ADDITIONAL_DIR_AUTHORITY=""
# TODO user should be able to set this
export APP_ELTOR_TOR_NICKNAME="elfdbe78324"
export APP_ELTOR_TOR_DATA_DIRECTORY="$APP_ELTOR_USER_DIR/data/tor"
export APP_ELTOR_TOR_SOCKS_PORT="0.0.0.0:18058"
export APP_ELTOR_TOR_CONTROL_PORT="9992"
# password1234_
export APP_ELTOR_TOR_HASHED_CONTROL_PASSWORD="16:281EC5644A4F548A60D50A0DD4DF835FFD50EDED062FD270D7269943DA"
export APP_ELTOR_TOR_CLIENT_ADDRESS="127.0.0.1"
export APP_ELTOR_TOR_PAYMENT_CIRCUIT_MAX_FEE="11000"
export APP_ELTOR_LN_CONFIG="$APP_ELTOR_LN_CONFIG"

######################################
### Relay Eltor Environment Variables
######################################
export APP_ELTOR_TOR_RELAY_ADDITIONAL_DIR_AUTHORITY="" 
export APP_ELTOR_TOR_RELAY_DATA_DIRECTORY="$APP_ELTOR_USER_DIR/data/tor-relay"
# public IP or domain of the relay (this is computed in the docker start.sh script)
# export APP_ELTOR_TOR_RELAY_ADDRESS="X.X.X.X"
export APP_ELTOR_TOR_RELAY_CONTACT="eltorcontact"
# TODO user should be able to set this
export APP_ELTOR_TOR_RELAY_NICKNAME="elr123r5"
export APP_ELTOR_TOR_RELAY_OR_PORT="9996"
export APP_ELTOR_TOR_RELAY_CONTROL_PORT="7781"
export APP_ELTOR_TOR_RELAY_SOCKS_PORT="18057"
# password1234_
export APP_ELTOR_TOR_RELAY_HASHED_CONTROL_PASSWORD="16:281EC5644A4F548A60D50A0DD4DF835FFD50EDED062FD270D7269943DA"
export APP_ELTOR_TOR_RELAY_SANDBOX="0"
export APP_ELTOR_TOR_EXIT_RELAY="1"
export APP_ELTOR_TOR_RELAY_PAYMENT_RATE_MSATS="1000"
export APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL="60"
export APP_ELTOR_TOR_RELAY_PAYMENT_INTERVAL_ROUNDS="10"
export APP_ELTOR_TOR_RELAY_PAYMENT_CIRCUIT_MAX_FEE="11000"
export APP_ELTOR_LN_BOLT12="$APP_ELTOR_LN_BOLT12"
export APP_ELTOR_LN_CONFIG="$APP_ELTOR_LN_CONFIG"
