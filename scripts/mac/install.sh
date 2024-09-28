#!/bin/bash

# 0. Backup the orginal tor binary and torrc file
mv /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_orig
mv ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc_orig

# 1. Download the tor binary and place it in the Tor Browser directory
echo "Downloading tor binary..."
curl -L -o /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/tor
if [ $? -ne 0 ]; then
  echo "Failed to download the tor binary."
  exit 1
fi
chmod +x /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor

# 2. Download the torrc file, rename it, and place it in the appropriate directory
echo "Downloading tor-browser-sample-torrc..."
curl -L -o ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/mac/torrc
if [ $? -ne 0 ]; then
  echo "Failed to download the torrc file."
  exit 1
fi

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


