#!/bin/sh -eu

BINARY=$(realpath ./target/debug/ds-identify)
INSTANCE=ds-test

lxc delete -f $INSTANCE || true
lxc init ubuntu-daily:lunar $INSTANCE
lxc file push "$BINARY" $INSTANCE/usr/lib/cloud-init/ds-identify

lxc start $INSTANCE

