#
# Make the base image configurable so that the E2E test can use a base image
# with a prepopulated Cargo build cache to accelerate the build process.
# Use Ubuntu 16.04 because this is what the Travis CI Krill build uses.
#
ARG BASE_IMG=alpine:3.11

#
# -- stage 1: build krill and krillc
#
FROM ${BASE_IMG} AS build

RUN apk add rust cargo openssl-dev

WORKDIR /tmp/krill
COPY . .

RUN cargo build --target x86_64-alpine-linux-musl --release

#
# -- stage 2: create an image containing just the binaries, configs &
#             scripts needed to run Krill, and not the things needed to build
#             it.
#
FROM alpine:3.11
COPY --from=build /tmp/krill/target/x86_64-alpine-linux-musl/release/krill /usr/local/bin/
COPY --from=build /tmp/krill/target/x86_64-alpine-linux-musl/release/krillc /usr/local/bin/

# Build variables for uid and guid of user to run container
ARG RUN_USER=krill
ARG RUN_USER_UID=1012
ARG RUN_USER_GID=1012

RUN apk add bash libgcc openssl tzdata util-linux

RUN addgroup -g ${RUN_USER_GID} ${RUN_USER} && \
    adduser -D -u ${RUN_USER_UID} -G ${RUN_USER} ${RUN_USER}

# Create the data directory structure and install a config file that uses it
WORKDIR /var/krill/data
COPY docker/krill.conf .
RUN chown -R ${RUN_USER}: .

# Install a Docker entrypoint script that will be executed when the container
# runs
COPY docker/entrypoint.sh /opt/
RUN chown ${RUN_USER}: /opt/entrypoint.sh

EXPOSE 3000/tcp

# Adding IPFS

#Install IPFS
WORKDIR /tmp/ipfs
RUN mkdir go-ipfs
COPY ./go-ipfs ./go-ipfs
RUN cd ./go-ipfs && ./install.sh
RUN apk add libc6-compat

# Expose ports
# Swarm TCP; should be exposed to the public
EXPOSE 4001
# Swarm UDP; should be exposed to the public
EXPOSE 4001/udp
# Daemon API; must not be exposed publicly but to client services under you control
EXPOSE 5001
# Web Gateway; can be exposed publicly with a proxy, e.g. as https://ipfs.example.org
EXPOSE 8080
# Swarm Websockets; must be exposed publicly when the node is listening using the websocket transport (/ipX/.../tcp/8081/ws).
EXPOSE 8081

# Create the fs-repo directory and switch to a non-privileged user.
ENV IPFS_PATH /data/ipfs
RUN mkdir -p $IPFS_PATH \
  && chown ${RUN_USER}:${RUN_USER_GID} $IPFS_PATH

# Create mount points for `ipfs mount` command
RUN mkdir /ipfs /ipns \
  && chown ${RUN_USER}:${RUN_USER_GID} /ipfs /ipns

# Expose the fs-repo as a volume.
# start_ipfs initializes an fs-repo if none is mounted.
# Important this happens after the USER directive so permissions are correct.
VOLUME $IPFS_PATH

# The default logging level
ENV IPFS_LOGGING ""

RUN echo $ENV_SWARM_KEY > $IPFS_PATH/swarm.key

RUN mkdir -p /usr/local/nexus \
    && chown ${RUN_USER}:${RUN_USER_GID} /usr/local/nexus

RUN touch /usr/local/nexus/peerid \
   && chown ${RUN_USER}:${RUN_USER_GID} /usr/local/nexus/peerid

RUN chmod 4755 /usr/local/nexus/peerid

# Use Tini to ensure that krillc responds to CTRL-C when run in the
# foreground without the Docker argument "--init" (which is actually another
# way of activating Tini, but cannot be enabled from inside the Docker image).
RUN apk add --no-cache tini
# Tini is now available at /sbin/tini
ENTRYPOINT ["/sbin/tini", "--", "/opt/entrypoint.sh"]
CMD ["krill", "-c", "/var/krill/data/krill.conf"]
