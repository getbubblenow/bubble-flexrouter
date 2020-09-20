#!/bin/bash

if [[ -z "${JOB_NAME}" ]] ; then
  echo "No JOB_NAME env var found"
fi
THISDIR=$(cd $(dirname ${0}) && pwd)

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
    MAKE_SYMLINKS=0
    SHA_CMD="shasum -a 256"
    ;;
  CYGWIN*)
    export PATH=${PATH}:/cygdrive/c/cygwin64/bin
    BUBBLE_DIST_HOME=${THISDIR}/dist
    rm -rf ${BUBBLE_DIST_HOME}/*
    MAKE_SYMLINKS=0
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
  echo "No version found in Cargo.toml"
  exit 1
fi
BUBBLE_VERSION=${BASE_VERSION}.${BUILD_NUMBER}

FLEX_DIST_TOP=${BUBBLE_DIST_HOME}/releases/bubble-flexrouter/${JOB_NAME}
FLEX_BINARY=$(find ${THISDIR}/target/release -type f -name "bubble-flexrouter*" | grep -v "bubble-flexrouter.d" | head -1)
if [[ -z "${FLEX_BINARY}" ]] ; then
  echo "No binary found in ${THISDIR}/target/release"
  exit 1
fi

FLEX_DIST=${FLEX_DIST_TOP}/${BUBBLE_VERSION}/bubble-flexrouter.zip
FLEX_DIST_DIR="$(dirname ${FLEX_DIST})"
if [[ ! -d "${FLEX_DIST_DIR}" ]] ; then
  mkdir -p ${FLEX_DIST_DIR}
fi

BUILD_DIR=${THISDIR}/build/bubble-flexrouter
cd ${THISDIR} && \
  mkdir -p ${BUILD_DIR} && \
  cp ${FLEX_BINARY} ${BUILD_DIR} && \
  cp README-release.md ${BUILD_DIR}/README.md && \
  cp flex_init.sh ${BUILD_DIR} && \
  cp flex_register.sh ${BUILD_DIR} && \
  echo "Building zip: ${FLEX_DIST}" && \
  cd build && zip -D -X -r ${FLEX_DIST} bubble-flexrouter
  cat ${FLEX_DIST} | ${SHA_CMD} | cut -f1 -d' ' | tr -d '\n' > ${FLEX_DIST}.sha256

if [[ ${MAKE_SYMLINKS} -eq 1 ]] ; then
  if [[ ${IS_DEV} -eq 0 ]] ; then
    ln -s ${FLEX_DIST} ${FLEX_DIST_DIR}/bubble-flexrouter.zip
    cd ${FLEX_DIST_TOP} && rm -f latest && ln -sf ${BUBBLE_VERSION} latest
    echo "${BUBBLE_VERSION}" > latest.txt
  fi
fi