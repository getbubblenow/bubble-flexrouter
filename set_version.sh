#!/bin/bash

if [[ -z "${BUILD_NUMBER}" ]] ; then
  echo "No BUILD_NUMBER environment variable was set"
  exit 1
fi
sed -i "s/.DEV_BUILD/.${BUILD_NUMBER}/" src/version.rs
