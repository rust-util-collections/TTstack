#!/bin/bash

###################################
# TESTing only, can NOT work now! #
###################################

modprobe vfio-pci

if_name="enp5s0"
if_id="0000:05:00.0"

vendor=$(cat /sys/bus/pci/devices/${if_id}/vendor)
device=$(cat /sys/bus/pci/devices/${if_id}/device)

echo "${if_id}" > /sys/bus/pci/devices/${if_id}/driver/unbind
echo "vfio-pci" > /sys/bus/pci/devices/${if_id}/driver_override
echo "${vendor} ${device}" > /sys/bus/pci/drivers/vfio-pci/new_id

echo "${if_id}" > /sys/bus/pci/drivers/vfio-pci/bind
