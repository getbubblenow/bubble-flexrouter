#!/bin/bash
#
# Utility to initialize a bubble-flexrouter.
#
# Note: Initialization requires the `htpasswd` program to be installed, to compute the bcrypted password
#
# Usage:
#
#      flex_init.sh [-f|--force] [-b|--bcrypt] [flex-password-env-var]
#
#   -f or --force         : Recreate token file, password file and SSH key, even if present
#
#   -b or --bcrypt        : If set, this means that the contents of the flex-password-env-var
#                           is a bcrypted password, not a plaintext password
#
#   flex-password-env-var : Name of environment variable containing password to bcrypt and write to password file
#                           Default value is BUBBLE_FR_PASS, or if not set, from prompt
#                           Ignored if password file already exists and -f / --force was not specified
#
#   The init command will create the following files:
#      ${FLEX_HOME}/.bfr_pass       # the bcrypted password
#      ${FLEX_HOME}/.bfr_token      # the flex router token
#      ${FLEX_HOME}/.ssh/flex.pub   # the SSH public key
#      ${FLEX_HOME}/.ssh/flex       # the SSH private key
#
# Environment variables:
#
#   FLEX_HOME : the base directory for files. Default is ${HOME}
#
SCRIPT="${0}"

function die {
  echo 1>&2 "${1}"
  exit 1
}

function log {
  echo 1>&2 "${SCRIPT} : ${1}"
}

case "$(uname -a | awk '{print $1}')" in
  Linux*)
    if [[ -z "${BUBBLE_DIST_HOME}" ]] ; then
      SHA_CMD="sha256sum"
    fi
    ;;
  Darwin*)
    SHA_CMD="shasum -a 256"
    ;;
  CYGWIN*)
    SHA_CMD="sha256sum"
    ;;
esac

function rand_string() {
  cat /dev/random | strings | head -c 1000 | ${SHA_CMD}
}

function write_ssh_key() {
  KEY_FILE="${1}"
  KEY_DIR="$(dirname ${KEY_FILE})"
  if [[ ! -d "${KEY_DIR}" ]] ; then
    mkdir -p "${KEY_DIR}" && chmod 700 ${KEY_DIR} || die "Error creating SSH key directory: ${KEY_DIR}"
  fi
  ssh-keygen -t rsa  -q -N '' -C 'bubble-flexrouter' -f ${KEY_FILE} || die "Error generating SSH key: ${KEY_FILE}"
}

log "Initializing flex-router"

FORCE=0
DO_BCRYPT=1

while [[ ! -z "${1}" && ${1} == -* ]] ; do
  if [[ ${1} == "--force" || ${1} == "-f" ]] ; then
    FORCE=1
    shift
  elif [[ ${1} == "--bcrypt" || ${1} == "-b" ]] ; then
    DO_BCRYPT=0
    shift
  else
    die "Only allowed options are: --force / -f and --bcrypt / -b"
  fi
done

if [[ -z "${FLEX_HOME}" ]] ; then
  FLEX_HOME="${HOME}"
fi
BFR_PASSWORD_FILE="${FLEX_HOME}/.bfr_pass"
BFR_TOKEN_FILE="${FLEX_HOME}/.bfr_token"
BFR_SSH_KEY_FILE="${FLEX_HOME}/.ssh/flex"

WRITE_PASS=0
if [[ -s ${BFR_PASSWORD_FILE} ]] ; then
  if [[ ${FORCE} -eq 1 ]] ; then
    log "Password file exists but -f / --force was set, overwriting: ${BFR_PASSWORD_FILE}"
    WRITE_PASS=1
  else
    log "Password file exists, not overwriting: ${BFR_PASSWORD_FILE}"
  fi
else
  WRITE_PASS=1
fi

if [[ ${WRITE_PASS} -eq 1 ]] ; then
  if [[ $DO_BCRYPT -eq 1 ]] ; then
    if [[ -z "$(which htpasswd)" ]] ; then
      die "htpasswd command not found, cannot bcrypt password"
    fi
  fi
  BFR_PASSWORD_VAR="${1}"
  if [[ -z "${BFR_PASSWORD_VAR}" ]] ; then
    BFR_PASSWORD_VAR="BUBBLE_FR_PASS"
  fi
  BFR_PASSWORD="${!BFR_PASSWORD_VAR}"
  if [[ -z "${BFR_PASSWORD}" ]] ; then
    read -sp "Bubble Flex Router Password: " BFR_PASSWORD
    # trim leading and trailing whitespace
    BFR_PASSWORD="$(echo -n "${BFR_PASSWORD}" | awk '{$1=$1};1')"
    if [[ -z "${BFR_PASSWORD}" ]] ; then
      die "No password set"
    fi
  fi
  if [[ $DO_BCRYPT -eq 1 ]] ; then
    echo "$(htpasswd -nbBC 12 USER "${BFR_PASSWORD}" | awk -F ':' '{print $2}')" > ${BFR_PASSWORD_FILE} || die "Error writing password file"
  else
    echo "${BFR_PASSWORD}" > ${BFR_PASSWORD_FILE} || die "Error writing password file"
  fi
  chmod 600 ${BFR_PASSWORD_FILE} || die "Error setting permission on password file: ${BFR_PASSWORD_FILE}"
  log "Wrote bcrypted password to ${BFR_PASSWORD_FILE}"
fi

if [[ -s ${BFR_TOKEN_FILE} ]] ; then
  if [[ ${FORCE} -eq 0 ]] ; then
    log "Token file exists, not overwriting: ${BFR_TOKEN_FILE}"
  else
    log "Token file exists but -f / --force was set, overwriting: ${BFR_TOKEN_FILE}"
    echo "$(rand_string)" > "${BFR_TOKEN_FILE}"
  fi
else
  log "Token file not found or empty, creating: ${BFR_TOKEN_FILE}"
  echo "$(rand_string)" > "${BFR_TOKEN_FILE}"
fi
chmod 600 ${BFR_TOKEN_FILE} || die "Error setting permission on token file: ${BFR_TOKEN_FILE}"

if [[ -s ${BFR_SSH_KEY_FILE} ]] ; then
  if [[ ${FORCE} -eq 0 ]] ; then
    log "SSH key file exists, not overwriting: ${BFR_SSH_KEY_FILE}"
  else
    log "SSH key file exists but -f / --force was set, overwriting: ${BFR_SSH_KEY_FILE}"
    rm -f ${BFR_SSH_KEY_FILE} ${BFR_SSH_KEY_FILE}.pub || die "Error removing existing key file: ${BFR_SSH_KEY_FILE} and ${BFR_SSH_KEY_FILE}.pub"
    write_ssh_key ${BFR_SSH_KEY_FILE}
  fi
else
  log "SSH key file not found or empty, creating: ${BFR_SSH_KEY_FILE}"
  write_ssh_key ${BFR_SSH_KEY_FILE}
fi

log "Initialization completed successfully"
