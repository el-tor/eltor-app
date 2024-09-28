#!/bin/bash

# 1. Download the tor binary and place it in the Tor Browser directory
echo "Downloading tor binary..."
mkdir ~/eltor
curl -L -o ~/eltor/tor https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/linux/tor
if [ $? -ne 0 ]; then
  echo "Failed to download the tor binary."
  exit 1
fi
chmod +x ~/eltor/tor

# 2. Download the torrc file, rename it, and place it in the appropriate directory
echo "Downloading torrc..."
curl -L -o ~/eltor/torrc.tmpl https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/linux/torrc_relay.tmpl
if [ $? -ne 0 ]; then
  echo "Failed to download the torrc file."
  exit 1
fi

cd ~/eltor

# 3. Config replacements in torrc template 
# Set default values
DEFAULT_NICKNAME="tormiddle"
DEFAULT_ADDRESS="127.0.0.1"
DEFAULT_ORPORT="5061"
DEFAULT_EXITRELAY="0"
# Prompt the user for input with default values
read -p "Enter the Nickname [default: $DEFAULT_NICKNAME]: " NICKNAME
NICKNAME=${NICKNAME:-$DEFAULT_NICKNAME}
read -p "Enter the Address (Public IP or hostname) [default: $DEFAULT_ADDRESS] find at https://www.whatsmyip.org/ : " ADDRESS
ADDRESS=${ADDRESS:-$DEFAULT_ADDRESS}
read -p "Enter the OrPort [default: $DEFAULT_ORPORT]: " ORPORT
ORPORT=${ORPORT:-$DEFAULT_ORPORT}
read -p "Enter the ExitRelay (0 for no, 1 for yes) [default: $DEFAULT_EXITRELAY]: " EXITRELAY

EXITRELAY=${EXITRELAY:-$DEFAULT_EXITRELAY}
TEMPLATE_FILE="torrc.tmpl"
OUTPUT_FILE="torrc"
# Use sed to replace placeholders in the template file
sed -e "s/Nickname tormiddle/Nickname $NICKNAME/" \
    -e "s/Address X.X.X.X/Address $ADDRESS/" \
    -e "s/OrPort 5061/OrPort $ORPORT/" \
    -e "s/ExitRelay 0/ExitRelay $EXITRELAY/" \
    "$TEMPLATE_FILE" > "$OUTPUT_FILE"

echo ""
echo ""
echo "
%@@@@@@@@@@@@@%%%@@@@%%%%%%%%%@@@@@@@@@@@@@@@@%%%%%@@@@@@%%%%%%%@@@@@@@@@@%%%%%%
@#***********#%%%****%%%%%%%@%********###**#%%%%@@%#++*#%@@%%%%%#*******##%@@%%%
@=:::::::::::-%@#::::#@%%%%@@#:::::::::::::-%@%@%+::::::-*%@%%%@+:::::::::-+%@%%
@=:::::::::::-@@#::::#@%%%%@@%:::::::::::::-%@@*::::::::::-#@%%@+:::::::::::-%%%
@=:::::::::::-@@#::::#@%%%%%@%:::::::::::::-@@+:::::::::::::#@%@+::::::::::::+@%
@=::::======-=%@#::::#@%%%%%@#-----::::----=@#::::::::::::::-%@@+::::-=--:::::#%
@=:::=@@@@@@@@@@#::::#@%%%%%%%@@@@#::::#@@@%%-::::-#%%%+:::::+@@+:::-%@@%+::::*@
@=:::=@@@@@@@@@@#::::#@%%%%@@@@%%@#::::#@%%@#::::-%@@@@@#::::-%@+:::-%@@@%::::+@
@=::::------*@@@#::::#@%%@%###%@%@#::::#@%%@+::::#@%%%%@@=::::%@+:::-@@@@*::::*@
@=::::::::::+@@@#::::#@%@%=--:+@%@#::::#@%%@+::::%@%%%%%@*::::#@+::::+++=:::::#@
@+::::::::::+@@@#::::#@%%%****#%%@#::::#@%%@+::::%@%%%%%@*::::#@+::::::::::::=%% RELAY
@+::::::::::+@@@#::::#@%%%@@@@@%%@#::::#@%%@+::::#@%%%%%@=::::%@+:::::::::::-%@%
@+:::=%%%%%%%@@@#::::#@%%%%@@%%%%@*::::#@%%@#::::-%@%%%@*::::-%@+::::::::---%@%%
@+:--=@%%%%@%%%@#:--:*%%%%%%%%%%%@*:--:#@%%%%-:--:-#%%%+:---:*@@+:---+*----:#@%%
@+:-----------+@#:-----------%@%%@*:--:#@%%%@#:--------:---:-%%@+:---%@+:----%%%
@+-----------:+@#:----------:#@%%@*:--:#@%%%%@*:----------:-%@%%+:---%@%----:+@%
@+------------+@#-----------:#%%%@*:--:#@%%%%%@*-:-------:=%@%%%+:---%%@#:----#@
@=:::--------:+@#::---:::--::#@%%@*::::#@%%%%%%@%+-------*%@%%%%+:::-%%%@*:--:-%
@############*#@%**#*********%%%%%#****%%%%%%%%%%@%#***#%@%%%%%%#****%%%%%**##*#
%@@@@@@@@@@@@@@%%@@%@@@@@%%@@%%%%%%@@@@%%%%%%%%%%%%@@@@%%%%%%%%%%@@@@%%%%%@@@%%%
"
echo ""
echo "El Tor Relay Configuration complete."
echo "Run the following command to start the relay (it could take over an hour until the relay is seen by the DA here http://93.127.216.111:7055/tor/status-vote/current/consensus):"
echo "sudo ufw allow 7061"
echo "cd ~/eltor"
echo "./tor -f torrc"
echo ""