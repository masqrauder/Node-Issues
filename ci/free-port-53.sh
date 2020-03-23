#!/bin/bash
command -v systemctrl && systemctl disable systemd-resolved.service
command -v service && service systemd-resolved stop
# [[ -e /etc/resolv.conf ]] && cat /etc/resolv.conf
[[ -f /etc/resolv.conf ]] && sed -E -e "s/127.0.0.53/1.1.1.1/g" -i '' /etc/resolv.conf
