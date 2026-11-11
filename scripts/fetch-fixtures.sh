#!/usr/bin/env bash
# Dumps the Metaplex Core program from devnet for the asset-signer (Core Execute) test.
# The binary is NOT committed (Metaplex license); it is fetched read-only for local tests.
set -euo pipefail
cd "$(dirname "$0")/../program-tests/tests/fixtures"
solana program dump CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d mpl_core_devnet.so --url devnet
