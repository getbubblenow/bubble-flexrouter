#!/bin/bash
#
# Called by jenkins to update version.rs with the current build number
#

if [[ -z "${BUILD_NUMBER}" ]] ; then
  echo "No BUILD_NUMBER environment variable was set"
  exit 1
fi
cp src/version.rs  src/version.rs.orig
cat src/version.rs.orig | sed "s/.DEV_BUILD/.${BUILD_NUMBER}/" > src/version.rs
