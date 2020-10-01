#!/bin/bash
#
# Uninstall bubble-flexrouter from a Mac OS X system
#

INSTALL_DIR="/Library/BubbleFlexrouter"
LAUNCH_DAEMON_NAME="com.bubble-vpn.flexrouter"
PLIST_FILE="${LAUNCH_DAEMON_NAME}.plist"
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
  echo "Started as $(whoami), running sudo"
  sudo "${0}"
  exit $?
fi

echo -n "Unloading service... "
INSTALLED="$(launchctl list ${LAUNCH_DAEMON_NAME} | wc -l | tr -d ' ')"
if [[ ${INSTALLED} -eq 0 ]] ; then
  echo "service not loaded"
else
  launchctl unload ${LAUNCH_DAEMON} || die "Error unloading service via: launchctl unload ${LAUNCH_DAEMON}"
  echo "OK"
fi

echo -n "Uninstalling service... "
if [[ -f "${LAUNCH_DAEMON}" ]] ; then
  rm -f "${LAUNCH_DAEMON}" || die "Error deleting ${LAUNCH_DAEMON}"
  echo "OK"
else
  echo "plist file not installed: ${LAUNCH_DAEMON}"
fi

echo -n "Deleting files... "
if [[ -d "${INSTALL_DIR}" ]] ; then
  rm -rf "${INSTALL_DIR}" || die "Error deleting ${INSTALL_DIR}"
  echo "OK"
else
  echo "No files found in ${INSTALL_DIR}"
fi

echo ""
echo "Uninstall completed successfully"
