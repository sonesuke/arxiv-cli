#!/bin/bash
set -e

ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
  NIX_SYSTEM="aarch64-linux"
else
  NIX_SYSTEM="x86_64-linux"
fi

NIX_FLAGS="--extra-experimental-features 'nix-command flakes'"

docker volume create nix-store 2>/dev/null || true

docker run --rm \
  -v "$(pwd):/workspace" \
  -v nix-store:/nix \
  -w /workspace \
  nixos/nix \
  sh -c "
    git config --global --add safe.directory /workspace
    nix $NIX_FLAGS build --no-link .#packages.${NIX_SYSTEM}.default
    cat \$(nix $NIX_FLAGS path-info .#packages.${NIX_SYSTEM}.default)
  " \
  | docker load
