#!/bin/bash

# 0. Backup the orginal tor binary and torrc file
mv ~/tor-browser/Browser/TorBrowser/Tor/tor ~/tor-browser/Browser/TorBrowser/Tor/tor_orig
mv ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc_orig

# 1. Download the tor binary and place it in the Tor Browser directory
echo "Downloading tor binary..."
curl -L -o ~/tor-browser/Browser/TorBrowser/Tor/tor https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/linux/tor
if [ $? -ne 0 ]; then
  echo "Failed to download the tor binary."
  exit 1
fi
chmod +x ~/tor-browser/Browser/TorBrowser/Tor/tor

# 2. Download the torrc file, rename it, and place it in the appropriate directory
echo "Downloading tor-browser-sample-torrc..."
curl -L -o ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc https://bitbucket.org/eltordev/eltor-app/raw/master/scripts/linux/torrc
if [ $? -ne 0 ]; then
  echo "Failed to download the torrc file."
  exit 1
fi

cd ~/tor-browser/Browser/TorBrowser/Data/Tor
rm cached-certs cached-microdesc-consensus cached-microdescs.new lock state


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
echo "El Tor binary drop in replacment copied to ~/tor-browser/Browser/TorBrowser/Tor/tor"
echo "El Tor torrc file downloaded and copied to ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc"
echo "El Tor Configuration complete."
echo "You can now open the Tor Browser and check you are connected by viewing the circuit built in the url bar points to 93.127.216.111"
echo ""
