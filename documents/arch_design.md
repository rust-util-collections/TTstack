# System Design

## 架构说明

### 上层逻辑

TT 中的基本管理单位为 “环境(ENV)”, 如下以时序图的方式展现一个 “环境” 的创建过程.

```mermaid
sequenceDiagram
    autonumber

    participant C as Client
    participant P as Proxy
    participant S as Server[s]
    participant R as Core

    C->>P: 创建 VM 的请求
    par 分发请求
    P-->>S: 计算可用资源, 并行分发
    end

    S->>R: 调用 Core 创建 VM
    Note right of S: 配置网络及生命周期
    R->>S: return

    par 返回结果
    S->>P: 异步返回
    end

    P->>C: 聚合各 Server 的结果

    loop 资源管理
    R-->>R: 定时清理过期的 VM
    end
```

### Core 内部实现

#### On Linux

```mermaid
sequenceDiagram
    autonumber

    participant C as Core
    participant K as Kernel
    participant Q as Qemu/FireCracker
    participant N as Nftables

    C->>K: 创建 PID NS 与 CGROUP
    C->>Q: 增量(COW)创建运行时镜像
    C->>N: 使用 Nftables 的哈希表结构管理 NAT 规则
```

## Why NOT

### Why NOT K8S

K8S 主要用于调度容器, 不适于对隔离性要求较高的场景.

### Why NOT OpenStack

OpenStack 太过复杂, 需要专门的团队维护, 成本太高.

### Why NOT Ansible

Ansible 只是一个批量管理工具, 不具备虚拟方案的管理与调度功能.

### Why NOT Libvirt

Libvirt 在安装系统及远程管理方面非常便捷, 但不具备自动化调度的能力. 当前 TT 系统使用 Libvirt 做为基础系统镜像的安装工具.
