#!/bin/bash
#
# Register a flex router with a Bubble.
#
# Note: Registration requires the `curl` and `jq` commands to be installed.
#
# Usage:
#
#     flex_register.sh bubble-hostname [flex-password-env-var]
#
#   bubble-hostname   : hostname of the bubble to register with
#
#   flex-password-env-var : Name of environment variable containing bubble-flexrouter password
#                           Default value is BUBBLE_FR_PASS
#
# Environment variables:
#
#   BUBBLE_USER       : username to login as. If empty, you'll be prompted for it.
#   BUBBLE_PASS       : password to login with. If empty, you'll be prompted for it.
#   BFR_ADMIN_PORT    : port on localhost where bubble-flexrouter admin API is listening. Default is 9833
#   BUBBLE_VPN_SUBNET : subnet prefix for VPN addresses. Default is "10.19."
#

LOGIN_JSON=$(mktemp /tmp/bubble_login.XXXXXXX.json)
chmod 600 ${LOGIN_JSON}

SESSION_FILE=$(mktemp /tmp/bubble_session.XXXXXXX.txt)
chmod 600 ${SESSION_FILE}

function die {
  echo 1>&2 "${1}"
  rm -f ${LOGIN_JSON} ${SESSION_FILE}
  exit 1
}

function vpn_ip() {
    SUBNET_MATCH=${1?no subnet match provided}
    case "$(uname -a | awk '{print $1}')" in
      Linux*)
        ip addr | grep inet | grep 10.19 | awk '{print $2}' | awk -F '/' '{print $1}'
        ;;
      Darwin*)
        ifconfig | grep inet | grep 10.19 | awk '{print $2}'
        ;;
      CYGWIN*)
        ipconfig | grep 10.19 | awk '{print $NF}'
        ;;
    esac
}

if [[ -z "$(which curl)" ]] ; then
  die "No curl command found on PATH"
fi
if [[ -z "$(which jq)" ]] ; then
  die "No jq command found on PATH"
fi

BUBBLE_HOSTNAME=${1:?no bubble-hostname provided}
BUBBLE_FR_PASS_ENV_VAR=${2:-BUBBLE_FR_PASS}

if [[ -z "${BFR_ADMIN_PORT}" ]] ; then
  BFR_ADMIN_PORT="9833"
fi
if [[ -z "${BUBBLE_VPN_SUBNET}" ]] ; then
  BUBBLE_VPN_SUBNET="10.19."
fi

FR_PASSWORD=${!BUBBLE_FR_PASS_ENV_VAR}
if [[ -z "${FR_PASSWORD}" ]] ; then
  die "bubble-flexrouter password environment variable was not defined or was empty: ${BUBBLE_FR_PASS_ENV_VAR}"
fi

BUBBLE_VPN_IP=$(vpn_ip ${BUBBLE_VPN_SUBNET})
if [[ -z "${BUBBLE_VPN_IP}" ]] ; then
  die "No VPN IP address found (expected something starting with ${BUBBLE_VPN_SUBNET}). Connect to your Bubble and try again."
fi

if [[ -z "${BUBBLE_USER}" ]] ; then
  read -p "Bubble Username: " BUBBLE_USER
fi

if [[ -z "${BUBBLE_USER}" ]] ; then
  die "No username provided"
fi

if [[ -z "${BUBBLE_PASS}" ]] ; then
  read -sp "Bubble Password: " BUBBLE_PASS
fi

if [[ -z "${BUBBLE_PASS}" ]] ; then
  die "No password provided"
fi

set -o pipefail
echo "{\"name\": \"${BUBBLE_USER}\", \"password\": \"${BUBBLE_PASS}\"}" > ${LOGIN_JSON} \
  && curl -s -H 'Content-Type: application/json' https://${BUBBLE_HOSTNAME}:1443/api/auth/login -d @${LOGIN_JSON} | jq -r .token > ${SESSION_FILE} \
  || die "Login error"
rm -f ${LOGIN_JSON}

if [[ ! -s ${SESSION_FILE} ]] ; then
  die "Login error: session not found in JSON response"
fi
SESSION_TOKEN="$(cat ${SESSION_FILE} | tr -d [[:space:]])"
echo "{
  \"password\": \"${FR_PASSWORD}\",
  \"session\": \"${SESSION_TOKEN}\",
  \"bubble\": \"${BUBBLE_HOSTNAME}\",
  \"ip\": \"${BUBBLE_VPN_IP}\"
}" > ${SESSION_FILE} && \
  curl -s -H 'Content-Type: application/json' http://127.0.0.1:${BFR_ADMIN_PORT}/register -d @${SESSION_FILE} \
  || die "Registration error"
rm -f ${SESSION_FILE}

echo "Registration successful"
