#!/bin/bash

if [[ -z "${JOB_NAME}" ]] ; then
  echo "No JOB_NAME env var found"
fi
THISDIR=$(cd $(dirname ${0}) && pwd)

function die {
  echo 1>&2 "${1}"
  exit 1
}

function platform_dist_zip {
  BUILD_DIR="${1}"
  PLIST_FILE="com.bubble-vpn.flexrouter.plist"
  case "$(uname -a | awk '{print $1}')" in
    Darwin*)
      cp ${THISDIR}/macos/${PLIST_FILE} ${BUILD_DIR} || die "Error copying macos files to BUILD_DIR: ${BUILD_DIR}"
      ;;
  esac
}

function platform_dist_files {
  BUBBLE_SHA="${1}"
  FLEX_DIST_DIR="${2}"
  case "$(uname -a | awk '{print $1}')" in
    Darwin*)
      cat ${THISDIR}/macos/install.sh \
      | sed -e "s/@@FR_DIST_VERSION@@/${BUBBLE_VERSION}/g" \
      | sed -e "s/@@FR_DIST_SHA@@/${BUBBLE_SHA}/g" \
      > ${FLEX_DIST_DIR}/install.sh || die "Error creating macos install.sh"
      cp ${THISDIR}/macos/uninstall.sh ${FLEX_DIST_DIR}/uninstall.sh || die "Error copying macos uninstall.sh"
      ;;
  esac
}

MAKE_SYMLINKS=0
case "$(uname -a | awk '{print $1}')" in
  Linux*)
    if [[ -z "${BUBBLE_DIST_HOME}" ]] ; then
      BUBBLE_DIST_HOME=${1:?no BUBBLE_DIST_HOME provided}
      MAKE_SYMLINKS=1
      SHA_CMD="sha256sum"
    fi
    ;;
  Darwin*)
    BUBBLE_DIST_HOME=${THISDIR}/dist
    rm -rf ${BUBBLE_DIST_HOME}/*
    SHA_CMD="shasum -a 256"
    ;;
  CYGWIN*)
    export PATH=${PATH}:/cygdrive/c/cygwin64/bin
    BUBBLE_DIST_HOME=${THISDIR}/dist
    rm -rf ${BUBBLE_DIST_HOME}/*
    SHA_CMD="sha256sum"
    ;;
esac

IS_DEV=0
if [[ -z ${BUILD_NUMBER} ]] ; then
  BUILD_NUMBER="dev"
  IS_DEV=1
fi
BASE_VERSION="$(cat ${THISDIR}/Cargo.toml | grep -m 1 version | awk -F '"' '{print $2}')"
if [[ -z ${BASE_VERSION} ]] ; then
  die "No version found in Cargo.toml"
fi
BUBBLE_VERSION=${BASE_VERSION}.${BUILD_NUMBER}

FLEX_DIST_TOP=${BUBBLE_DIST_HOME}/releases/bubble-flexrouter/${JOB_NAME}
FLEX_BINARY=$(find ${THISDIR}/target/release -type f -name "bubble-flexrouter*" | grep -v "bubble-flexrouter.d" | head -1)
if [[ -z "${FLEX_BINARY}" ]] ; then
  die "No binary found in ${THISDIR}/target/release"
fi

FLEX_DIST=${FLEX_DIST_TOP}/${BUBBLE_VERSION}/bubble-flexrouter.zip
FLEX_DIST_DIR="$(dirname ${FLEX_DIST})"
if [[ ! -d "${FLEX_DIST_DIR}" ]] ; then
  mkdir -p ${FLEX_DIST_DIR} || die "Error creating FLEX_DIST_DIR: ${FLEX_DIST_DIR}"
fi

BUILD_DIR=${THISDIR}/build/bubble-flexrouter
cd ${THISDIR} && \
  mkdir -p ${BUILD_DIR} && \
  cp ${FLEX_BINARY} ${BUILD_DIR} && \
  cp README-release.md ${BUILD_DIR}/README.md && \
  cp flex_init.sh ${BUILD_DIR} && \
  cp flex_register.sh ${BUILD_DIR} && \
  platform_dist_zip ${BUILD_DIR} && \
  echo "Building zip: ${FLEX_DIST}" && \
  cd build && zip -D -X -r ${FLEX_DIST} bubble-flexrouter || die "Error building bubble-flexrouter dist zip file"

cat ${FLEX_DIST} | ${SHA_CMD} | cut -f1 -d' ' | tr -d '\n' > ${FLEX_DIST}.sha256 || die "Error calculating SHA for bubble-flexrouter dist zip file"
platform_dist_files "$(cat ${FLEX_DIST}.sha256)" ${FLEX_DIST_DIR}

if [[ ${MAKE_SYMLINKS} -eq 1 ]] ; then
  if [[ ${IS_DEV} -eq 0 ]] ; then
    ln -s ${FLEX_DIST} ${FLEX_DIST_DIR}/bubble-flexrouter.zip || die "Error creating bubble-flexrouter latest symlink"
    cd ${FLEX_DIST_TOP} && rm -f latest && ln -sf ${BUBBLE_VERSION} latest || die "Error creating bubble-flexrouter latest dir symlink"
    echo "${BUBBLE_VERSION}" > latest.txt  || die "Error creating bubble-flexrouter latest.txt file"
  fi
fi
