#!/bin/bash

if [[ -z "${JOB_NAME}" ]] ; then
  echo "No JOB_NAME env var found"
fi
THISDIR=$(cd $(dirname ${0}) && pwd)

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

if [[ -z "${BUBBLE_DIST_HOME}" ]] ; then
  BUBBLE_DIST_HOME=${1:?no BUBBLE_DIST_HOME provided}
fi

FLEX_DIST_TOP=${BUBBLE_DIST_HOME}/releases/bubble-flexrouter/${JOB_NAME}
FLEX_BINARY=$(find ${THISDIR}/target/release -type f -name "bubble-flexrouter*" | grep -v "bubble-flexrouter.d" | head -1)
if [[ -z "${FLEX_BINARY}" ]] ; then
  echo "No binary found in target/release"
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
  cd build && zip -D -X -r bubble-flexrouter ${FLEX_DIST}
  cat ${FLEX_DIST} | sha256sum | cut -f1 -d' ' | tr -d '\n' > ${FLEX_DIST}.sha256

if [[ ${IS_DEV} -eq 0 ]] ; then
  cd ${FLEX_DIST_TOP} && rm -f latest && ln -sf ${BUBBLE_VERSION} latest
  echo "${BUBBLE_VERSION}" > latest.txt
fi
