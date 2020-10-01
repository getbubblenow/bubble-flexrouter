#!/bin/bash
#
# Install bubble-flexrouter on Mac OS X as a LaunchDaemon
#
FR_DIST_TMP_DIR=$(mktemp -d /tmp/bubble-flexrouter.XXXXXXX)
FR_DIST_TMP_ZIP=${FR_DIST_TMP_DIR}/bubble-flexrouter.zip

INSTALL_DIR="/Library/BubbleFlexrouter"
PLIST_FILE="com.bubble-vpn.flexrouter.plist"
LAUNCH_DAEMON="/Library/LaunchDaemons/${PLIST_FILE}"

function die {
  echo 1>&2 "${1}"
  rm -rf ${FR_DIST_TMP_DIR}
  exit 1
}

PLATFORM="$(uname -a | awk '{print $1}')"
case "${PLATFORM}" in
  Darwin*)
    # OK
    ;;
  *)
    die "This is the Mac OS X install.sh script. Cannot run on ${PLATFORM}"
    ;;
esac

if [[ $(whoami) != "root" ]] ; then
  if [[ -z "${0}" || "${0}" == "bash" || "${0}" == "/bin/bash" ]] ; then
    die "Must be run using sudo"
  fi
  echo "Started as $(whoami), running sudo"
  THIS_DIR="$(cd $(dirname ${0}) && pwd)"
  sudo bash "${THIS_DIR}/${0}"
  exit $?
fi

# Set by jenkins when dist zip file is created
FR_DIST_VERSION="@@FR_DIST_VERSION@@"
FR_DIST_SHA="@@FR_DIST_SHA@@"

FR_DIST_URL="https://jenkins.bubblev.org/public/releases/bubble-flexrouter/bubble-flexrouter-macos/@@FR_DIST_VERSION@@/bubble-flexrouter.zip"

# Download the zip file and check the SHA
echo -n "Downloading... "
curl ${FR_DIST_URL} > ${FR_DIST_TMP_ZIP} || die "Error downloading flexrouter zip file from ${FR_DIST_URL}"
if [[ ! -s ${FR_DIST_TMP_ZIP} ]] ; then
  die "Error downloading flexrouter, downloaded file does not exist or is empty"
fi
echo "OK"

echo -n "Verifying... "
FR_ACTUAL_SHA="$(cat ${FR_DIST_TMP_ZIP} | shasum -a 256 | awk '{print $1}')"
if [[ "${FR_ACTUAL_SHA}" != "${FR_DIST_SHA}" ]] ; then
  die "SHA-256 sum did not match. Found ${FR_ACTUAL_SHA} but expected ${FR_DIST_SHA} for ${FR_DIST_URL}"
fi
echo "OK"

# Unzip archive to temp dir
echo -n "Unpacking... "
cd ${FR_DIST_TMP_DIR} && unzip ${FR_DIST_TMP_ZIP} || die "Error unzipping bubble-flexrouter.zip"
echo "OK"

echo -n "Installing files.... "
mkdir -p ${INSTALL_DIR} || die "Error creating directory: ${INSTALL_DIR}"
cp ${FR_DIST_TMP_DIR}/bubble-flexrouter/* ${INSTALL_DIR} || die "Error copying files to directory: ${INSTALL_DIR}"
echo "OK"

echo "Initializing.... "
export FLEX_HOME="${INSTALL_DIR}"
${INSTALL_DIR}/flex_init.sh || die "Error initializing flexrouter with flex_init.sh"
echo "Initialized"

echo -n "Installing service... "
cp ${INSTALL_DIR}/${PLIST_FILE} ${LAUNCH_DAEMON} || die "Error copying ${PLIST_FILE} -> ${LAUNCH_DAEMON}"
launchctl load ${LAUNCH_DAEMON} || die "Error installing service via launchctl"
echo "OK"

echo -n "Cleaning up temporary files... "
rm -rf ${FR_DIST_TMP_DIR}
echo "OK"

echo ""
echo "bubble-flexrouter successfully installed"
