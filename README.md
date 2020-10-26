# tt

[![pipeline status](https://gitlab.com/ktmlm/tt/badges/master/pipeline.svg)](https://gitlab.com/ktmlm/tt/-/commits/master)
[![coverage report](https://gitlab.com/ktmlm/tt/badges/master/coverage.svg)](https://gitlab.com/ktmlm/tt/-/commits/master)

A light-weight 'private cloud solution' for SMEs, it can bring huge help and commercial value to start-up companies.

面向中小型企业的轻量级私有云平台, 可快速生成各种虚拟机环境, 为产品兼容性验证和自动化测试等场景提供高效的基础环境.

## 中小企业的效率悖论

初创的中小企业, 技术实力薄弱, 很多生产手段都停留在刀耕火种的蛮荒时代.

**这就形成了一个悖论:**

> 原理上来讲, 初创公司要赶超大型公司, 必须要赢在效率; 确实, 在"人"的主观效率上, 如"管理流程"等方面, 大多数初创公司因为业务场景简单, 都能做到这一点；但"技术流程"上, 却是落后地一塌糊涂, 其结果当然是惨不忍睹, 尤如二战中的波兰骑兵, 高扬着马刀(原始工具)冲向德国人的坦克(现代工具), 不管马背上的骑士如何迅捷("人"的效率高), 都干不过坐在坦克("工具"的效率高)中的德国兵.
>
> 同时, 由于初创公司资金短缺, 很少有第三方的公司愿意去开拓这一块市场(无利可图); 而初创公司本身, 又没有足够的资源去自己解决, 这样就进入一个恶性循环, 永远处在"头痛医头, 脚痛医脚"的低效状态 , 直到公司倒闭, 或出现某个牛人以一己之力改变现状.

本项目以简洁易用为宗旨, 志在解决这个"无人问津"的悖论:

- 专门面向中小企业设计, 分布式架构, 可扩展, 可伸缩
- 充分利用硬件资源: 通过云平台统一调度所有硬件资源, 大幅提升资源利用率和灵活性
- 极低的系统架设和维护成本: 运维人员通常只需半小时即可搭建起一套完整的 TT 私有云平台
- 极低的学习和使用成本: 终端用户通常只需十分钟即可熟练使用 TT 客户端创建需要的虚拟环境
- **省钱, 是的! 很省钱!** 你无需耗费巨资养活一个专门的云团队(OpenStack/K8S 专业人员的身价通常都很高)
- 公有云真的很便宜? 很便利? 很安全? 用过的都知道答案
- ...

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

## RoadMap

- [-] 完善英文文档
- [-] 优化客户端的用户提示信息
- [-] 添加配套的视频教程
- [-] 添加配套的前端界面
- [-] 支持对已创建的 ENV 打快照
    - `tt env snapshot <ENV> ...`

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
        --image-path /home/images \
        --cpu-total 2 \
        --mem-total $[4 * 1024 * 1024] \
        --disk-total $[40 * 1024 * 1024] \
        --serv-addr 127.0.0.1 \
        --serv-port 20000

# Slave Server 2
ttserver \
        --image-path /home/images \
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
> - "/home/images" 路径下需要存在可正常启动的 Qemu 镜像文件
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

## Statistics

```
(git)-[master]-% tokei
===============================================================================
 Language            Files        Lines         Code     Comments       Blanks
===============================================================================
 BASH                    1            5            2            1            2
 Makefile                1          108           92            0           16
 Shell                   8          278          198           30           50
 TOML                    9          224          187            1           36
-------------------------------------------------------------------------------
 Markdown               15          654            0          466          188
 |- Shell                4          371          332           23           16
 (Total)                           1025          332          489          204
-------------------------------------------------------------------------------
 Rust                   77         8565         7228          260         1077
 |- Markdown            71          741           41          649           51
 (Total)                           9306         7269          909         1128
===============================================================================
 Total                 111        10946         8080         1430         1436
===============================================================================
```
