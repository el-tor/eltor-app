#!/bin/bash

if [ -f ~/tor-browser/Browser/TorBrowser/Tor/tor_orig ]; then
  mv ~/tor-browser/Browser/TorBrowser/Tor/tor ~/tor-browser/Browser/TorBrowser/Tor/tor_el
  mv ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc_el
  mv ~/tor-browser/Browser/TorBrowser/Tor/tor_orig ~/tor-browser/Browser/TorBrowser/Tor/tor
  mv ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc_orig ~/tor-browser/Browser/TorBrowser/Data/Tor/torrc
  cd ~/tor-browser/Browser/TorBrowser/Data/Tor
  rm cached-certs cached-microdesc-consensus cached-microdescs.new lock state
  echo "Disconnected from the El Tor network. Tor Browser should be connected to the normal Tor network."
else
  echo "Cannot uninstall. File does not exist ~/tor-browser/Browser/TorBrowser/Tor/tor_orig"
fi