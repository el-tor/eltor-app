{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    
    # Build dependencies
    pkg-config
    openssl
    sqlite
    autoconf
    automake
    libtool
    gnumake
    wget
    git
    flex
    bison
    unzip
    gh
    
    # macOS specific
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];
  
  # Environment variables
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.sqlite.dev}/lib/pkgconfig";
  SQLITE3_LIB_DIR = "${pkgs.sqlite.out}/lib";
  SQLITE3_INCLUDE_DIR = "${pkgs.sqlite.dev}/include";
  
  # Rust target - automatically detect based on platform
  CARGO_BUILD_TARGET = 
    if pkgs.stdenv.hostPlatform.system == "aarch64-darwin" then "aarch64-apple-darwin"
    else if pkgs.stdenv.hostPlatform.system == "x86_64-darwin" then "x86_64-apple-darwin"
    else if pkgs.stdenv.hostPlatform.system == "aarch64-linux" then "aarch64-unknown-linux-gnu"
    else if pkgs.stdenv.hostPlatform.system == "x86_64-linux" then "x86_64-unknown-linux-gnu"
    else pkgs.stdenv.hostPlatform.system;

  shellHook = ''
    if [ -f .secrets ]; then
      set -a
      source .secrets
      set +a
    fi
  '';
}
