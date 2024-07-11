FROM golang:1.22-alpine AS go-builder

SHELL ["/bin/sh", "-ecuo", "pipefail"]

RUN apk add --no-cache ca-certificates build-base git

WORKDIR /code/server

# Copy over code, do this earlier due to ethos-avs dependency
COPY server/ /code/server/

# force it to use static lib (from above) not standard libgo_cosmwasm.so file
# then log output of file /code/bin/ethosd
# then ensure static linking
RUN LEDGER_ENABLED=false BUILD_TAGS=muslc LINK_STATICALLY=true make build \
  && file /code/server/build/server \
  && echo "Ensuring binary is statically linked ..." \
  && (file /code/server/build/server | grep "statically linked")

# --------------------------------------------------------
FROM debian:10.13

RUN apt-get update && apt-get install -y gpg curl make jq apt-transport-https software-properties-common wget

RUN wget https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -O /usr/bin/yq &&\
        chmod +x /usr/bin/yq

COPY --from=go-builder /code/server/build/server /usr/bin/server

WORKDIR /opt

# set local IP address of machine running Ethereum node
ARG LOCAL_IP
ENV LOCAL_IP=${LOCAL_IP}

# grpc & gateway
EXPOSE 50051 8080

CMD ["/usr/bin/server", "version"]