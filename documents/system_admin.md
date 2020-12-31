# TT 系统管理员指南

Guide for system admin.

## 环境准备

> If use zfs, should do this: `zfs create -V 1M zroot/tt/__sample__`.

镜像相关:
- [Linux, MUST] `systemctl disable NetworkManager`
- [Linux, Optional] `systemctl disable firewalld`
- [Linux, Optional] `sed -i 's/SELINUX=.*/SELINUX=disabled/' /etc/selinux/config`
- 保证本项目定制的 "[rc.local](../tools/images/linux_vm/rc.local)" 文件开机启动
- [ FireCracker ] 内核模块只能 `strip --strip-debug`, strip 过度会导致无法载入
- [ FireCracker ] 关闭内核的模块签名校验功能, 因为签名信息存在于已被 strip 的 debug 信息中
- [ FireCracker ] `firecracker` 二进制文件要使用 `musl-libc` 编译
    - 由于 `Rust` 动态链接 `glibc` , 故运行时存在更多的不确定性因素, 参见: [issue #2044](https://github.com/firecracker-microvm/firecracker/issues/2044)

#### On Linux

> 使用 Qemu\FireCracker 做为 VM 引擎, 使用 Nftables 做 NAT 端口转发.
>
> - `firecracker` 程序路径必须是 `/usr/sbin/firecracker`

环境配置:

- `modprobe tun vhost_net`

组件安装

- `qemu`
- `nftables`
- `ZOL: zfs on linux`

## ttserver 配置

```shell
USAGE:
    ttserver [FLAGS] [OPTIONS]

FLAGS:
    -h, --help         Prints help information
    -V, --version      Prints version information

OPTIONS:
        --cpu-total <NUM>      可以使用的 CPU 核心总数.
        --disk-total <SIZE>    可以使用的磁盘总量, 单位: MB.
        --image-path <PATH>    镜像存放路径.
        --cfgdb-path <PATH>    ENV 配置信息存放路径.
        --log-path <PATH>      日志存储路径.
        --mem-total <SIZE>     可以使用的内存总量, 单位: MB.
        --serv-addr <ADDR>     服务监听地址.
        --serv-port <PORT>     服务监听端口.
```

## ttproxy 配置

```shell
USAGE:
    ttproxy [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --proxy-addr <ADDR>       ttproxy 地址, eg: 127.0.0.1:19527.
        --server-set <ADDR>... ttserver 地址, eg: 127.0.0.1:9527,10.10.10.101:9527.
```

## gitlab CI/CD

shell 模式下, 需要将 `~gitlab-runner/.bash_logout` 删除:

```shell
rm ~gitlab-runner/.bash_logout
```
