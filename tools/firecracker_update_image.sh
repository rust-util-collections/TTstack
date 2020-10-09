#!/usr/bin/env bash

zfspath="zroot/tt"
if [[ "" != $1 ]]; then
    zfspath=$1
fi

oslist="
    fire:centos-7.x:3.10.0-693.el7.x86_64
    fire:centos-7.x:3.10.0-862.el7.x86_64
    fire:centos-7.x:3.10.0-957.el7.x86_64
    fire:centos-7.x:3.10.0-1062.el7.x86_64
    fire:centos-7.x:3.10.0-1127.el7.x86_64

    fire:centos-7.x:4.14.0-115.el7.0.1.x86_64

    fire:centos-8.x:4.18.0-80.el8.x86_64
    fire:centos-8.x:4.18.0-80.1.2.el8.x86_64
    fire:centos-8.x:4.18.0-80.4.2.el8.x86_64
    fire:centos-8.x:4.18.0-80.7.1.el8.x86_64
    fire:centos-8.x:4.18.0-80.7.2.el8.x86_64
    fire:centos-8.x:4.18.0-80.11.1.el8.x86_64
    fire:centos-8.x:4.18.0-80.11.2.el8.x86_64

    fire:centos-8.x:4.18.0-147.el8.x86_64
    fire:centos-8.x:4.18.0-147.0.3.el8.x86_64
    fire:centos-8.x:4.18.0-147.3.1.el8.x86_64
    fire:centos-8.x:4.18.0-147.5.1.el8.x86_64
    fire:centos-8.x:4.18.0-147.8.1.el8.x86_64

    fire:centos-8.x:4.18.0-193.el8.x86_64
    fire:centos-8.x:4.18.0-193.1.2.el8.x86_64
    fire:centos-8.x:4.18.0-193.6.3.el8.x86_64
    fire:centos-8.x:4.18.0-193.14.2.el8.x86_64
    fire:centos-8.x:4.18.0-193.19.1.el8.x86_64
"

oslist_dev="
    fire:dev:4.19.148.x86_64
"

oslist_bad="
    fire:centos-7.x:3.10.0-123.el7.x86_64
    fire:centos-7.x:3.10.0-229.el7.x86_64
    fire:centos-7.x:3.10.0-327.el7.x86_64
    fire:centos-7.x:3.10.0-514.el7.x86_64
"

destroy_old() {
    for x in $(echo ${oslist}); do
        zfs destroy -R ${zfspath}/${x}@base
    done

    for x in $(echo ${oslist_dev}); do
        zfs destroy -R ${zfspath}/${x}@base
    done

    for x in $(echo ${oslist_bad}); do
        zfs destroy -R ${zfspath}/${x}@base 2>/dev/null
    done

    zfs destroy -R ${zfspath}/firecracker@base
    zfs destroy -R ${zfspath}/firecracker-dev@base
}

create_new() {
    zfs snapshot ${zfspath}/firecracker@base || exit 1
    for x in $(echo ${oslist}); do
        zfs clone ${zfspath}/firecracker@base ${zfspath}/$x || exit 1
        zfs snapshot ${zfspath}/${x}@base || exit 1
    done

    zfs snapshot ${zfspath}/firecracker-dev@base || exit 1
    for x in $(echo ${oslist_dev}); do
        zfs clone ${zfspath}/firecracker-dev@base ${zfspath}/${x} || exit 1
        zfs snapshot ${zfspath}/${x}@base || exit 1
    done
}

destroy
destroy_old
create_new
