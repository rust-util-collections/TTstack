# tt

[![pipeline status](http://192.168.3.189/fanhui/tt/badges/master/pipeline.svg)](http://192.168.3.189/fanhui/tt/-/commits/master)
[![coverage report](http://192.168.3.189/fanhui/tt/badges/master/coverage.svg)](http://192.168.3.189/fanhui/tt/-/commits/master)

tt, Temporary Test.

面向中小型企业的轻量级私有云平台, 可快速生成各种虚拟机环境, 为产品兼容性验证和自动化测试提供高效的基础环境.

## Why you will need this ?

- 专门面向中小企业设计, 分布式架构, 可扩展, 可伸缩
- 充分利用硬件资源: 通过云平台统一调度所有硬件资源, 大幅提升资源利用率和灵活性
- 极低的系统架设和维护成本: 运维人员通常只需半小时即可搭建起一套完整的 TT 私有云平台
- 极低的学习和使用成本: 终端用户通常只需十分钟即可熟练使用 TT 客户端创建需要的虚拟环境
- **省钱, 是的! 很省钱!** 你无需耗费巨资养活一个专门的云团队(OpenStack/K8S 专业人员的身价通常都很高)
- 公有云真的很便宜? 很便利? 很安全? 用过的都知道答案
- ...

## Quick Start

#### 编译

```shell
make install
export PATH=~/.cargo/bin:$PATH
```

#### 启动服务端

> **注意**
>
> 镜像文件一定**不**能存放在'/tmp'或其子目录下, 会导致无法扫描到镜像信息(ttserver 的'/tmp'路径私有的, 与外界环境互相隔离).

```shell
# Slave Server 1
ttserver \
        --image-path /tmp/images \
        --cpu-total 2 \
        --mem-total $[4 * 1024 * 1024] \
        --disk-total $[40 * 1024 * 1024] \
        --serv-addr 127.0.0.1 \
        --serv-port 20000

# Slave Server 2
ttserver \
        --image-path /tmp/images \
        --cpu-total 2 \
        --mem-total $[4 * 1024 * 1024] \
        --disk-total $[40 * 1024 * 1024] \
        --serv-addr 127.0.0.1 \
        --serv-port 20001

# Proxy, 分布式代理服务, 负责调度各 Slave Server 的资源
ttproxy \
        --proxy-addr 127.0.0.1:20002 \
        --server-set 127.0.0.1:20000,127.0.0.1:20001
```

#### 客户端操作

> **Tips**
> - 完整的客户端操作文档, 参见: [《用户指南》](./documents/user_guide.md)
> - "/tmp/images" 路径下需要存在可正常启动的 Qemu 镜像文件
> - 镜像文件中的 "/etc/rc.local" 文件需要替换为本项目定制的 "[rc.local](./tools/images/linux_vm/rc.local)"

```shell
# 配置服务端地址,
# 既可以是 Proxy 的地址,
# 也可以是各个独立的 Slave Server 地址,
# 这里配置成 Proxy 的地址, 以演示分布式架构的调度效果
tt config --serv-addr 127.0.0.1 --serv-port 20002

# 查看客户端本地信息
tt status

# 查看服务端资源信息
tt status --server

# 创建一个 "ENV",
# TT 中的基本管理单位为 ENV (一组 VM 的集合),
# 创建的 VM 类别是以系统前缀匹配的, 不区分大小写,
# 如:
#     - cent 会匹配到所有 CentOS 系统
#     - ubuntu2004 只会匹配到 Ubuntu2004 一个系统
tt env add TEST --os-prefix=cent,ubuntu2004

# 查看已创建的 ENV 列表
tt env list

# 查看已创建的某个 ENV 的详情
tt env show TEST

# 在 ENV 的所有 VM 上执行相同的命令
tt env run TEST --use-ssh --cmd 'ls /'

# 删除 ENV,
# 其中所有的 VM 及其相关数据都会被清理
tt env del TEST
```

## 主要用途

1. 广泛的平台兼容性验证
    - 可在如下两个方向上做任意的交叉组合
        1. Linux、BSD、Windows、MacOS 等各种 OS 类别与版本
        2. AMD64、X86、AArch64、ARM、MIPS、RISC-V、SPARC 等各种硬件平台
2. 与 DevOps 系统配合, 实现自动化的 CI\CD 功能
3. 用作原生编译平台
    - 直接申请全量的原生 OS 环境, 避免交叉编译的复杂度和潜在问题
4. 用作短期或长期的调试环境
    - 可将 TT 视为云平台, 申请虚拟机用于开发和测试
5. 其它...

## 技术特性

- 整洁高效的资源管理
    - 每个 VM 存在于独立的 Cgroup 中, 资源清理准确无误
    - [可选] 使用 FireCracker 快速创建大量的轻量级 MicroVM
    - [默认] 使用 zfs 的 `snapshot\clone` 机制使 VM 获得原生 IO 性能
    - [默认] 使用 nftables 的 `SET\MAP` 等高级数据结构管理网络端口
    - 服务进程运行在单独的 `PID NS` 中, 服务退出会自动销毁所有资源
    - 通过 `Rust Drop` 机制自动管理 VM 生命周期
    - ...
- 分布式可扩展架构
    - 后端支持多机分布式架构, 对用户完全透明
- 轻量级的通信模型
    - C\S 两端基于 UDP\SCTP 进行通信
    - 自研的远程命令执行工具, 效率远超 SSH 协议
- 镜像源与服务解耦
    - 可随时增加受支持的系统镜像, 服务端不需要停机
    - 支持多种虚拟机引擎, 如: Qemu\FireCracker\Bhyve 等
    - 以镜像名称前缀识别虚拟机类型, 如: fire:centos-7.3:3.10.e17.x86_64
- 使用`Rust`语言开发
    - 安全稳定
    - 高效运行
    - 文档齐备
    - 原生跨平台
    - ...

## 详细文档

- [终端用户指南](./documents/user_guide.md)
- [系统管理指南](./documents/system_admin.md)
- [架构设计与技术选型](./documents/arch_design.md)
- [项目结构与代码规模](./documents/code_about.md)

> #### 接口文档
>
> ```shell
> # 在 Rust 开发环境下执行
> make doc
> ```

## BUG

- 单个 ENV 超过 400+ VM 时, 可能出现异常
    - 原因是 UDP 单次通信最多承载 64K 的数据量, ENV 太大会超限
    - 目前采用了数据压缩的方式予以缓解, 后续将改用 SCTP 或直接使用 HTTP

