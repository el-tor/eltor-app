#!/bin/bash

#if [ -f /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_orig ]; then
  mv /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_el
  mv ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc_el
  mv /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_orig /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor
  mv ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc_orig ~/Library/Application\ Support/TorBrowser-Data/Tor/torrc
  echo "Removed El Tor. Tor Browser should be connected to the normal Tor network."
#else
#  echo "Cannot uninstall. File does not exist /Applications/Tor\ Browser.app/Contents/MacOS/Tor/tor_orig"
#fi

