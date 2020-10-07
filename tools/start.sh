#!/usr/bin/env bash

#################################################
#### Ensure we are in the right path. ###########
#################################################
if [[ 0 -eq `echo $0 | grep -c '^/'` ]]; then
    # relative path
    EXEC_PATH=$(dirname "`pwd`/$0")
else
    # absolute path
    EXEC_PATH=$(dirname "$0")
fi

EXEC_PATH=$(echo ${EXEC_PATH} | sed 's@/\./@/@g' | sed 's@/\.*$@@')
cd $EXEC_PATH || exit 1
#################################################

export PS4='+(${BASH_SOURCE}:${LINENO}): ${FUNCNAME[0]:+${FUNCNAME[0]}(): }'
export PATH=/usr/local/bin:$PATH

ip=$1
port=$2
imgpath=$3

if [[ "" == $imgpath || "" == $ip ]]; then
    syskind=$(uname -s)
    if [[ "Linux" == $syskind ]]; then
        ip=$(ip addr | grep -Eo '192\.168\.[0-9]{1,3}\.[0-9]{1,3}' | head -1)
        imgpath="/data/images"
    elif [[ "FreeBSD" == $syskind ]]; then
        ip=$(ifconfig | grep -Eo '192\.168\.[0-9]{1,3}\.[0-9]{1,3}' | head -1)
        imgpath="/dev/zvol/zroot/bhyve"
    fi
fi

if [[ "" == $port ]]; then
    port=9527
fi

nohup ttserver \
    --serv-addr=${ip} \
    --serv-port=${port} \
    --image-path=${imgpath} \
    --cpu-total=144 \
    --mem-total=54244 \
    --disk-total=8192000 &
