# images

运行时镜像相关的说明.

## rc.local

各镜像内部均已预置本路径下的`rc.local`开机启动项.

#### 示例

客户机`MAC`地址的末尾两段数字转化为十进制后, 作为`IP`地址的后两段使用.

因此为虚拟机设置`MAC`地址的同时, 会自动设置其`IP`地址.

```shell
# MAC 地址
f4:b5:20:1b:3a:83

# IP 地址
10.10.58.131
```

## Linux Images

```shell
# tree -F
.
├── centos/
│   ├── qemu:CentOS-7.0.qcow2:default
│   ├── qemu:CentOS-7.1.qcow2:default
│   ├── qemu:CentOS-7.2.qcow2:default
│   ├── qemu:CentOS-7.3.qcow2:default
│   ├── qemu:CentOS-7.4.qcow2:default
│   ├── qemu:CentOS-7.5.qcow2:default
│   ├── qemu:CentOS-7.6.qcow2:default
│   ├── qemu:CentOS-7.7.qcow2:default
│   ├── qemu:CentOS-7.8.qcow2:default
│   └── qemu:CentOS-8.2.qcow2:default
└── ubuntu/
    ├── qemu:ubuntu-14.04.qcow2:default
    ├── qemu:ubuntu-16.04.qcow2:default
    ├── qemu:ubuntu-18.04.qcow2:default
    └── qemu:ubuntu-20.04.qcow2:default
```
