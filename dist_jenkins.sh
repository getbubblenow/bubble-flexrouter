#!/bin/bash

FLEX_PROJECT=${1:?no project name specified}

JENKINS_HOME="$(cd ~jenkins && pwd)"

function die () {
  echo 1>&2 "${1}"
  exit 1
}

IS_MACOS=0
case "${FLEX_PROJECT}" in
  *macos*)
    IS_MACOS=1
  ;;
esac

LATEST_BUILD="$(find ${JENKINS_HOME}/jobs/${FLEX_PROJECT}/builds -maxdepth 1 -mindepth 1 -type d | xargs -n 1 basename  | sort -nr | head -1)"
if [[ -z "${LATEST_BUILD}" ]] ; then
  die "No latest build found"
fi

LATEST_VERSION="$(find ${JENKINS_HOME}/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/dist/releases/bubble-flexrouter/${FLEX_PROJECT} -maxdepth 1 -mindepth 1 -type d | sort -nr | head -1 | xargs -n 1 basename)"
if [[ -z "${LATEST_VERSION}" ]] ; then
  die "No latest version found"
fi

LATEST_ZIP="$(find ${JENKINS_HOME}/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/dist/releases/bubble-flexrouter/${FLEX_PROJECT}/${LATEST_VERSION} -maxdepth 1 -mindepth 1 -type f -name "bubble-flexrouter.zip" | head -1)"
if [[ -z "${LATEST_ZIP}" ]] ; then
  die "No latest zip found"
fi

if [[ ${IS_MACOS} -eq 1 ]] ; then
  LATEST_INSTALL_SH="${JENKINS_HOME}/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/install.sh"
  if [[ ! -s "${LATEST_INSTALL_SH}" ]] ; then
    die "No install.sh found"
  fi
  LATEST_UNINSTALL_SH="${JENKINS_HOME}/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/uninstall.sh"
  if [[ ! -s "${LATEST_UNINSTALL_SH}" ]] ; then
    die "No uninstall.sh found"
  fi
fi

RELEASE_TOP="${JENKINS_HOME}/public/public/releases/bubble-flexrouter/${FLEX_PROJECT}/"
RELEASE_DIR="${RELEASE_TOP}/${LATEST_VERSION}"

mkdir -p ${RELEASE_DIR} || die "Error creating release dir: ${RELEASE_DIR}"
echo "Created release dir: ${RELEASE_DIR}"

cp ${LATEST_ZIP} ${RELEASE_DIR} || die "Error copying ${LATEST_ZIP} -> ${RELEASE_DIR}"
cp ${LATEST_ZIP}.sha256 ${RELEASE_DIR} || die "Error copying ${LATEST_ZIP}.sha256 -> ${RELEASE_DIR}"
if [[ ${IS_MACOS} -eq 1 ]] ; then
  cp ${LATEST_INSTALL_SH} ${RELEASE_DIR} || die "Error copying ${LATEST_INSTALL_SH} -> ${RELEASE_DIR}"
  cp ${LATEST_UNINSTALL_SH} ${RELEASE_DIR} || die "Error copying ${LATEST_UNINSTALL_SH} -> ${RELEASE_DIR}"
fi
echo "Published release: ${RELEASE_DIR}/$(basename ${LATEST_ZIP})"

echo ${LATEST_VERSION} > ${RELEASE_TOP}/latest.txt
echo "Marked as latest release: ${RELEASE_TOP}/latest.txt == $(cat ${RELEASE_TOP}/latest.txt)"

cd ${RELEASE_TOP} && rm -f latest && ln -s $(basename ${RELEASE_DIR}) latest
echo "Marked as latest release: ${RELEASE_TOP}/latest -> $(basename ${RELEASE_DIR})"
