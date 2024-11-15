#!/bin/bash

# 0. Backup the orginal tor binary and torrc file
mv /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_orig
mv ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc_orig


# 1. eltor directory
mkdir ~/eltor
mkdir ~/eltor/chutney
mkdir ~/eltor/chutney/tor-proxy
mkdir ~/eltor/chutney/tor-proxy/tor
mkdir ~/eltor/chutney/tor-proxy/eltor


# 2. Download the tor binary and place it in the Tor Browser directory
echo "Downloading tor binary..."
curl -L -o /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/eltor
if [ $? -ne 0 ]; then
  echo "Failed to download the tor binary."
  exit 1
fi
chmod +x /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor
cp /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor ~/eltor/chutney/tor-proxy/eltor/tor

# 3. Download the torrc file, rename it, and place it in the appropriate directory
echo "Downloading tor-browser-sample-torrc..."
curl -L -o ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/torrc-eltor-proxy
if [ $? -ne 0 ]; then
  echo "Failed to download the torrc file."
  exit 1
fi
cp ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc ~/eltor/chutney/tor-proxy/eltor/torrc

# 4 Download regular tor proxy
curl -L -o ~/eltor/chutney/tor-proxy/tor/tor https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/tor
curl -L -o ~/eltor/chutney/tor-proxy/tor/torrc https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/torrc-tor-proxy
chmod +x ~/eltor/chutney/tor-proxy/tor/tor

# 5. Install haproxy
# TODO prereq us homebrew
brew install haproxy
curl -L -o ~/eltor/chutney/tor-proxy/tor/haproxy.cfg https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/haproxy.cfg


# 6. Start the proxy services
# Function to kill background processes
# cleanup() {
#   echo "Cleaning up..."
#   kill $TOR1_PID $TOR2_PID $HAPROXY_PID
# }
# # Trap signals and call cleanup
# trap cleanup SIGINT SIGTERM
# cd ~/eltor/chutney/tor-proxy/eltor
# ./tor -f torrc &
# TOR1_PID=$!
# cd ~/eltor/chutney/tor-proxy/tor
# ./tor -f torrc &
# TOR2_PID=$!
# haproxy -f haproxy.cfg &
# HAPROXY_PID=$!

# 7. Open the browser with the tor socks proxy
# TODO let user choose the browser to proxy tor thru
# "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --proxy-server="socks5://127.0.0.1:1080"
# CHROME_PID=$!
# echo "Started Chrome with PID $CHROME_PID"

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
@+::::::::::+@@@#::::#@%%%****#%%@#::::#@%%@+::::%@%%%%%@*::::#@+::::::::::::=%%
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
echo "El Tor binary drop in replacment copied to /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor"
echo "El Tor torrc file downloaded and copied to ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc"
echo "El Tor Configuration complete."
echo "You can now open the Tor Browser and check you are connected by viewing the circuit built in the url bar points to 93.127.216.111"
echo ""


# # Monitor Chrome process
# while kill -0 $CHROME_PID 2> /dev/null; do
#   sleep 1
# done

# # If Chrome process ends, clean up
# cleanup