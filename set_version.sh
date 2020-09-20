#!/bin/bash
#
# Called by jenkins to update version.rs with the current build number
#

if [[ -z "${BUILD_NUMBER}" ]] ; then
  echo "No BUILD_NUMBER environment variable was set"
  exit 1
fi

THISDIR=$(cd $(dirname ${0}) && pwd)
BASE_VERSION="$(cat ${THISDIR}/Cargo.toml | grep -m 1 version | awk -F '"' '{print $2}')"
if [[ -z ${BASE_VERSION} ]] ; then
  echo "No version found in Cargo.toml"
  exit 1
fi
cp src/version.rs  src/version.rs.orig
cat src/version.rs.orig \
  | sed "s/FLEX_VERSION./${BASE_VERSION}./" \
  | sed "s/.DEV_BUILD/.${BUILD_NUMBER}/" \
  > src/version.rs
