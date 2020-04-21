#!/bin/bash -evx
command -v systemctl && systemctl stop systemd-resolved
command -v systemctl && systemctl disable systemd-resolved.service
command -v service && service systemd-resolved stop
# [[ -e /etc/resolv.conf ]] && cat /etc/resolv.conf
[[ -f /etc/systemd/resolved.conf ]] && sed -E -e 's/#DNS=$/DNS=1.1.1.1/g' -e 's/#DNSStubListener=yes$/DNSStubListener=no/g' -i '' /etc/systemd/resolved.conf || echo 'Edit Complete'
[[ -f /run/systemd/resolve/resolv.conf && -L /etc/resolv.conf ]] && ln -sf /run/systemd/resolve/resolv.conf /etc/resolv.conf
