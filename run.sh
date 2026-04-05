#!/bin/sh
exec systemd-inhibit --what=idle:sleep cargo run -- "$@"
