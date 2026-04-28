#!/usr/bin/env bash
set -euo pipefail

conda install -y -c conda-forge libnetcdf pkg-config

{
  echo "NETCDF_DIR=${CONDA_PREFIX}/Library"
  echo "PKG_CONFIG_PATH=${CONDA_PREFIX}/Library/lib/pkgconfig"
  echo "RUSTFLAGS=-L native=${CONDA_PREFIX}/Library/lib"
  echo "INCLUDE=${CONDA_PREFIX}/Library/include"
  echo "LIB=${CONDA_PREFIX}/Library/lib"
} >> "${GITHUB_ENV}"
