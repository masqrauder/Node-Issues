#!/bin/bash -evx
command -v systemctrl && systemctl disable systemd-resolved.service
command -v service && service systemd-resolved stop
# [[ -e /etc/resolv.conf ]] && cat /etc/resolv.conf
[[ -f /etc/systemd/resolv.conf ]] && sed -E -e -E -e "s/#DNS=$/DNS=1.1.1.1/g
s/#DNSStubListener=yes$/DNSStubListener=no/g" -i '' /etc/systemd/resolv.conf
[[ -f /etc/systemd/resolve/resolv.conf && -L /etc/resolv.conf ]] && ln -sf /run/systemd/resolve/resolv.conf /etc/resolv.conf
