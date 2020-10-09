#!/usr/bin/env bash

name="muslenv"

if [[ 0 == $(docker ps --filter="name=^${name}$" | grep -c "${name}") ]]; then
    docker run -v $(pwd):/volume --name $name --privileged --rm -dt clux/muslrust
fi
