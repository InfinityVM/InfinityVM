#!/bin/sh

node_exporter &

exec /usr/local/bin/reth
