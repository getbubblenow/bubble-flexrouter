#!/bin/bash
#
# Install bubble-flexrouter on Mac OS X as a LaunchDaemon
#
FR_DIST_TMP_DIR=$(mktemp -d /tmp/bubble-flexrouter.XXXXXXX)
FR_DIST_TMP_ZIP=${FR_DIST_TMP_DIR}/bubble-flexrouter.zip

function die {
  echo 1>&2 "${1}"
  rm -rf ${FR_DIST_TMP_DIR}
  exit 1
}

# Set by jenkins when dist zip file is created
FR_DIST_VERSION="@@FR_DIST_VERSION@@"
FR_DIST_SHA="@@FR_DIST_SHA@@"

FR_DIST_URL="https://jenkins.bubblev.org/public/releases/bubble-flexrouter/bubble-flexrouter-macos/@@FR_DIST_VERSION@@/bubble-flexrouter.zip"

# Download the zip file and check the SHA
echo "Downloading bubble-flexrouter..."
curl -s ${FR_DIST_URL} > ${FR_DIST_TMP_ZIP} || die "Error downloading flexrouter zip file from ${FR_DIST_URL}"
FR_ACTUAL_SHA="$(cat ${FR_DIST_TMP} | shasum -a 256 | awk '{print $2}')"
if [[ "${FR_ACTUAL_SHA}" != "${FR_DIST_SHA}" ]] ; then
  die "SHA-256 sum did not match. Found ${FR_ACTUAL_SHA} but expected ${FR_DIST_SHA} for ${FR_DIST_URL}"
fi

# Unzip archive to temp dir
echo "Unpacking bubble-flexrouter..."
cd ${FR_DIST_TMP_DIR} && unzip ${FR_DIST_TMP_ZIP} || die "Error unzipping bubble-flexrouter.zip"

cd bubble-flexrouter

# Clean up
# rm -rf ${FR_DIST_TMP_DIR}

echo "bubble-flexrouter successfully installed"
