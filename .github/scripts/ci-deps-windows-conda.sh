#!/usr/bin/env bash
# Sourced from ci.yml (same bash -el process as setup-miniconda) so `conda` is on PATH.
# If executed directly, try to load conda for Windows GitHub Actions.
set -euo pipefail

if ! command -v conda >/dev/null 2>&1; then
  if [[ -n "${CONDA:-}" && -f "${CONDA}/etc/profile.d/conda.sh" ]]; then
    # shellcheck source=/dev/null
    source "${CONDA}/etc/profile.d/conda.sh"
  fi
fi
if ! command -v conda >/dev/null 2>&1; then
  echo "conda not found; ensure setup-miniconda ran in a prior step." >&2
  exit 127
fi

# Pin hdf5 to 1.x: hdf5-metno-sys (via zahirscan/netcdf stack) rejects H5_VERSION 2.x
# (CI saw Invalid H5_VERSION: "2.2.0" from conda-forge's current default).
conda install -y -c conda-forge "hdf5>=1.14,<2" libnetcdf pkg-config

{
  echo "NETCDF_DIR=${CONDA_PREFIX}/Library"
  echo "PKG_CONFIG_PATH=${CONDA_PREFIX}/Library/lib/pkgconfig"
  echo "RUSTFLAGS=-L native=${CONDA_PREFIX}/Library/lib"
  echo "INCLUDE=${CONDA_PREFIX}/Library/include"
  echo "LIB=${CONDA_PREFIX}/Library/lib"
} >> "${GITHUB_ENV}"
