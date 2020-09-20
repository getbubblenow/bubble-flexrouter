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
#                           Default value is BUBBLE_FR_PASS
#                           Ignored if password file already exists and -f / --force was not specified
#
#   The init command will create the following files:
#      ${HOME}/.bfr_pass       # the bcrypted password
#      ${HOME}/.bfr_token      # the flex router token
#      ${HOME}/.ssh/flex.pub   # the SSH public key
#      ${HOME}/.ssh/flex       # the SSH private key
#

function die {
  echo 1>&2 "${1}"
  exit 1
}

function rand_string() {
  LEN=${1:-50}
  cat /dev/random | strings | tr -d [[:space:]] | head -c ${LEN}
}

function write_ssh_key() {
  KEY_FILE="${1}"
  KEY_DIR="$(dirname ${KEY_FILE})"
  if [[ ! -d "${KEY_DIR}" ]] ; then
    mkdir -p "${KEY_DIR}" && chmod 700 ${KEY_DIR} || die "Error creating SSH key directory: ${KEY_DIR}"
  fi
  ssh-keygen -t rsa  -q -N '' -C 'bubble-flexrouter' -f ${KEY_FILE} || die "Error generating SSH key: ${KEY_FILE}"
}

echo "Initializing flex-router"

FORCE=0
DO_BCRYPT=1

while [[ ! -z "${1}" && ${1} == -* ]] ; then
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

BFR_PASSWORD_FILE="${HOME}/.bfr_pass"
BFR_TOKEN_FILE="${HOME}/.bfr_token"
BFR_SSH_KEY_FILE="${HOME}/.ssh/flex"

WRITE_PASS=0
if [[ -s ${BFR_PASSWORD_FILE} ]] ; then
  if [[ ${FORCE} -eq 1 ]] ; then
    echo "Password file exists but -f / --force was set, overwriting: ${BFR_PASSWORD_FILE}"
    WRITE_PASS=1
  else
    echo "Password file exists, not overwriting: ${BFR_PASSWORD_FILE}"
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
    die "Environment variable ${BFR_PASSWORD_VAR} was not defined or was empty"
  fi
  if [[ $DO_BCRYPT -eq 1 ]] ; then
    echo "$(htpasswd -nbBC 12 USER "${BFR_PASSWORD}" | awk -F ':' '{print $2}')" > ${BFR_PASSWORD_FILE} || die "Error writing password file"
  else
    echo "${BFR_PASSWORD}" > ${BFR_PASSWORD_FILE} || die "Error writing password file"
  fi
  chmod 600 ${BFR_PASSWORD_FILE} || die "Error setting permission on password file: ${BFR_PASSWORD_FILE}"
  echo "Wrote bcrypted password to ${BFR_PASSWORD_FILE}"
fi

if [[ -s ${BFR_TOKEN_FILE} ]] ; then
  if [[ ${FORCE} -eq 0 ]] ; then
    echo "Token file exists, not overwriting: ${BFR_TOKEN_FILE}"
  else
    echo "Token file exists but -f / --force was set, overwriting: ${BFR_TOKEN_FILE}"
    echo "$(rand_string)" > "${BFR_TOKEN_FILE}"
  fi
else
  echo "Token file not found or empty, creating: ${BFR_TOKEN_FILE}"
  echo "$(rand_string)" > "${BFR_TOKEN_FILE}"
fi
chmod 600 ${BFR_TOKEN_FILE} || die "Error setting permission on token file: ${BFR_TOKEN_FILE}"

if [[ -s ${BFR_SSH_KEY_FILE} ]] ; then
  if [[ ${FORCE} -eq 0 ]] ; then
    echo "SSH key file exists, not overwriting: ${BFR_SSH_KEY_FILE}"
  else
    rm -f ${BFR_SSH_KEY_FILE} ${BFR_SSH_KEY_FILE}.pub || die "Error removing existing key file: ${BFR_SSH_KEY_FILE} and ${BFR_SSH_KEY_FILE}.pub"
    write_ssh_key ${BFR_SSH_KEY_FILE}
  fi
else
  write_ssh_key ${BFR_SSH_KEY_FILE}
fi

echo "Initialization completed successfully"