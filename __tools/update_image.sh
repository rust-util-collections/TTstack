#!/bin/sh

zvol_prefix="/dev/zvol/zroot/tt-nftables/"
tap_path="/tmp/.xxsssxxxyyyyyyxx__xxx"

echo '#!/bin/sh
ip link set $1 up; sleep 1; ip link set $1 master ttcore-bridge' > $tap_path
chmod +x $tap_path

let i=0
for img in Alpine-3.12 CentOS-6.10 CentOS-7.0 CentOS-7.1 CentOS-7.2 CentOS-7.3 \
            CentOS-7.4 CentOS-7.5 CentOS-7.6 CentOS-7.7 CentOS-7.8 CentOS-8.2 \
            Ubuntu-1410 Ubuntu-1504 Ubuntu-1510 Ubuntu-1604 Ubuntu-1610 Ubuntu-1704 Ubuntu-1710 \
            Ubuntu-1804 Ubuntu-1810 Ubuntu-1904 Ubuntu-1910 Ubuntu-2004 slow-Ubuntu-1404;
do
    let i+=1
    id=$(printf "%02x" ${i})

    qemu-system-x86_64 -enable-kvm -cpu host -smp 2 -m 800 \
            -netdev tap,ifname=TMP_${i}-if,script=${tap_path},downscript=no,id=TMP_${i}-NIC \
            -device virtio-net-pci,mac=00:be:fa:76:09:${id},netdev=TMP_${i}-NIC \
            -drive file=${zvol_prefix}${img},if=none,format=raw,cache=none,id=TMP_${i}-DISK \
            -device virtio-blk-pci,drive=TMP_${i}-DISK -boot order=cd -daemonize -vnc :$i
done

sleep 120

data1="/data/ftp_home/tt_releases/linux/ttrexec-daemon"
data2="/data/ftp_home/tt_releases/linux/ttrexec-daemon"

for ((;i>0;i--)); do
    ssh -i ~/.ssh/tt_rsa root@10.10.9.${i} pkill -9 ttrexec

    scp -i ~/.ssh/tt_rsa $data1 root@10.10.9.${i}:/usr/local/bin/
    scp -i ~/.ssh/tt_rsa $data2 root@10.10.9.${i}:/usr/local/bin/

    ssh -i ~/.ssh/tt_rsa root@10.10.9.${i} poweroff
done
