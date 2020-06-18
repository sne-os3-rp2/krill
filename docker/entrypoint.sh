#!/bin/bash
# Prepare the environment and config file for the Krill daemon.
# This script supports several scenarios:
#   A. The operator wants to run the Krill daemon using the default setup:
#      We have to fix a couple of things before running the Krill daemon:
#        - Krill doesn't know the FQDN at which it's HTTPS, RSYNC and RRDP
#          endpoints are published but needs to include that FQDN in data that
#          it produces. Configure it based on env var KRILL_FQDN.
#        - Krill doesn't have a default API token value, we have to supply one.
#          Generate one and announce it, if no KRILL_AUTH_TOKEN env var was
#          supplied by the operator.
#   
#   B: The operator wants to control the Krill daemon configuration themselves.
#      They do this by Docker mounting their own krill.conf over the
#      /var/krill/data/krill.conf path.
#
#   C: The operator wants to run some other command in the container, e.g.
#      krill_admin.
#
set -e
KRILL_CONF=/var/krill/data/krill.conf
KRILL_FQDN="${KRILL_FQDN:-localhost:3000}"
KRILL_AUTH_TOKEN="${KRILL_AUTH_TOKEN:-None}"
KRILL_LOG_LEVEL="${KRILL_LOG_LEVEL:-warn}"
KRILL_USE_TA="${KRILL_USE_TA:-false}"

MAGIC="# DO NOT TOUCH, THIS LINE IS MANAGED BY DOCKER KRILL"
LOG_PREFIX="docker-krill:"

log_warning() {
    echo >&2 "${LOG_PREFIX} Warning! $*"
}

log_info() {
    echo "${LOG_PREFIX} $*"
}

if [ "$1" == "krill" ]; then
    # Does the opreator want to use their own API token? If so they must
    # supply the KRILL_AUTH_TOKEN env var.
    if [ "${KRILL_AUTH_TOKEN}" == "None" ]; then
        # Generate a unique hard to guess authorisation token and export it
        # so that the Krill daemon uses it (unless overriden by the Krill
        # daemon config file). Only do this if the operator didn't already
        # supply a token when launching the Docker container.
        export KRILL_AUTH_TOKEN=$(uuidgen)
    fi

    # Announce the token in the Docker logs so that clients can obtain it.
    log_info "Securing Krill daemon with token ${KRILL_AUTH_TOKEN}"

    log_info "Configuring ${KRILL_CONF} .."
    # If the config file was persisted and the container was recreated with
    # different arguments to docker run there may still be some lines in the
    # config file that we added before which are now no longer correct. Remove
    # any lines that we added.
    if ! sed -i "/.\\+${MAGIC}/d" ${KRILL_CONF} 2>/dev/null; then
        log_warning "Cannot write to ${KRILL_CONF}. You can ignore this warning if you mounted your own config file over ${KRILL_CONF}."
    else
        # Append to the default Krill config file to direct clients of the
        # RSYNC and RRDP endpoints to the correct FQDN. We cannot know know the
        # FQDN which clients use to reach us so the operator must inform this
        # script via a "-e KRILL_FQDN=some.domain.name" argument to
        # "docker run".
        cat << EOF >> ${KRILL_CONF}
rsync_base  = "rsync://${KRILL_FQDN}/repo/" ${MAGIC}
service_uri = "https://${KRILL_FQDN}/" ${MAGIC}
log_level   = "${KRILL_LOG_LEVEL}" ${MAGIC}
use_ta      = ${KRILL_USE_TA} ${MAGIC}
EOF

        log_info "Dumping ${KRILL_CONF} config file"
        cat ${KRILL_CONF}
        log_info "End of dump"
    fi
fi

# Prepare IPFS

set -e
repo="$IPFS_PATH"

if [ -e "$repo/config" ]; then
  echo "Found IPFS fs-repo at $repo"
else
  case "$IPFS_PROFILE" in
    "") INIT_ARGS="" ;;
    *) INIT_ARGS="--profile=badgerds" ;;
  esac
  ipfs init $INIT_ARGS
  ipfs config Addresses.API /ip4/0.0.0.0/tcp/5001
  ipfs config Addresses.Gateway /ip4/0.0.0.0/tcp/8080

# Set up the swarm key, if provided

  SWARM_KEY_FILE="$repo/swarm.key"
  SWARM_KEY_PERM=0400

  # Create a swarm key from a given environment variable
  if [ ! -z "$IPFS_SWARM_KEY" ] ; then
    echo "Copying swarm key from variable..."
    echo -e "$IPFS_SWARM_KEY" >"$SWARM_KEY_FILE" || exit 1
    chmod $SWARM_KEY_PERM "$SWARM_KEY_FILE"
  fi

  # Unset the swarm key variable
  unset IPFS_SWARM_KEY

  # Check during initialization if a swarm key was provided and
  # copy it to the ipfs directory with the right permissions
  # WARNING: This will replace the swarm key if it exists
  if [ ! -z "$IPFS_SWARM_KEY_FILE" ] ; then
    echo "Copying swarm key from file..."
    install -m $SWARM_KEY_PERM "$IPFS_SWARM_KEY_FILE" "$SWARM_KEY_FILE" || exit 1
  fi

  # Unset the swarm key file variable
  unset IPFS_SWARM_KEY_FILE

fi

# Second guard rail to ensure a private network
ipfs bootstrap rm --all

# if node is bootnode then output the peerId
echo "Running as $(whoami)"

if [ ! -z "$IS_BOOTNODE" ] ; then
     echo "Copying peer id of boot node..."
     PEER_ID=$(ipfs id | grep "ID" | cut -d ':' -f 2 | sed 's/.$//' | tr -d '"' | tr -d " ")
     echo "Peer ID ${PEER_ID} generated"
     cat /dev/null > /usr/local/nexus/peerid
     echo $PEER_ID > /usr/local/nexus/peerid
     echo "Saved peer ID of bootnode as:"
     echo $(cat /usr/local/nexus/peerid)
else
     PEER_ID=$(cat /usr/local/nexus/peerid)
     echo "Reading peer id ${PEER_ID} of boot node..."
fi

ipfs bootstrap add /ip4/${BOOTNODE_IP}/tcp/4001/ipfs/${PEER_ID}

ipfs daemon --migrate=true --enable-namesys-pubsub --enable-pubsub-experiment &



# Launch the command supplied either by the default CMD (krill) in the
# Dockerfile or that given by the operator when invoking Docker run. Use exec
# to ensure krill runs as PID 1 as required by Docker for proper signal
# handling. This also allows this Docker image to be used to run krill_admin
# instead of krill.
exec "$@"
