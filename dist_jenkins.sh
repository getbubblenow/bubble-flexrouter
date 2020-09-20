#!/bin/bash

FLEX_PROJECT=${1:?no project name specified}

function die () {
  echo 1>&2 "${1}"
  exit 1
}

LATEST_BUILD="$(find ~/jobs/${FLEX_PROJECT}/builds -maxdepth 1 -mindepth 1 -type d | xargs -n 1 basename  | sort -nr | head -1)"
if [[ -z "${LATEST_BUILD}" ]] ; then
  die "No latest build found"
fi

LATEST_VERSION="$(find ~/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/dist/releases/bubble-flexrouter/${FLEX_PROJECT} -maxdepth 1 -mindepth 1 -type d | sort -nr | head -1 | xargs -n 1 basename)"
if [[ -z "${LATEST_VERSION}" ]] ; then
  die "No latest version found"
fi

LATEST_ZIP="$(find ~/jobs/${FLEX_PROJECT}/builds/${LATEST_BUILD}/archive/dist/releases/bubble-flexrouter/${FLEX_PROJECT}/${LATEST_VERSION} -maxdepth 1 -mindepth 1 -type f -name "bubble-flexrouter.zip" | head -1)"
if [[ -z "${LATEST_ZIP}" ]] ; then
  die "No latest zip found"
fi

RELEASE_TOP="~jenkins/public/public/releases/bubble-flexrouter/${FLEX_PROJECT}/"
RELEASE_DIR="${RELEASE_TOP}/${LATEST_VERSION}"

mkdir -p ${RELEASE_DIR} || die "Error creating release dir: ${RELEASE_DIR}"
cp ${LATEST_ZIP} ${RELEASE_DIR} || die "Error copying ${LATEST_ZIP} -> ${RELEASE_DIR}"
echo ${LATEST_VERSION} > ${RELEASE_TOP}/latest.txt
cd ${RELEASE_TOP} && rm -f latest && ln -s $(basename ${RELEASE_DIR}) latest
