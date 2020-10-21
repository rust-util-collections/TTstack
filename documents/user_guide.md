# tt client

用户用来与 TT 服务端进行交互的工具.

> **仅支持 Linux 与 MacOS**

## 安装

```shell
sudo make install
chmod +x /usr/local/bin/tt
```

## 使用

```shell
USAGE:
    tt [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    config
    env
    help      Prints this message or the help of the given subcommand(s)
    status
```

#### 配置 tt 服务端的地址

##### 命令参数

> ```shell
> tt-config
>
> USAGE:
>     tt config [OPTIONS]
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
>
> OPTIONS:
>     -n, --client-id <NAME>      客户端别名.
>     -a, --server-addr <ADDR>    服务端的监听地址.
>     -p, --server-port <PORT>    服务端的监听端口.
> ```

##### 示例

> ```shell
> tt config --server-addr=10.10.10.22 [--server-port=9527]
> ```

#### 创建环境

##### 命令参数

> ```shell
> tt-env-add
>
> USAGE:
>     tt env add <ENV> [FLAGS] [OPTIONS]
>     tt env add [FLAGS] [OPTIONS] -- <ENV>
>
> FLAGS:
>     -n, --deny-outgoing    禁止虚拟机对外连网.
>     -h, --help             Prints help information
>     -V, --version          Prints version information
>
> OPTIONS:
>     -C, --cpu-num <CPU_SIZE>       虚拟机的 CPU 核心数量.
>     -D, --disk-size <DISK_SIZE>    虚拟机的磁盘容量, 单位: MB.
>     -d, --dup-each <NUM>           每种虚拟机类型启动的实例数量.
>     -l, --life-time <TIME>         虚拟机的生命周期, 单位: 秒.
>     -M, --mem-size <MEM_SIZE>      虚拟机的内存容量, 单位: MB.
>     -s, --os-prefix <OS>... 虚拟机的系统, 如: CentOS7.x 等.
>     -p, --vm-port <PORT>... 虚拟机需要开放的网络端口.
>
> ARGS:
>     <ENV>    待创建的环境名称.
> ```

注意:

- 若不指定`--cpu-num`, CPU 默认为 2 Core
- 若不指定`--mem-size`, 内存默认为1024 MB
- 若不指定`--vm-port`, 默认只开放 22/62222 端口
- 若不指定`--dup-each`, 默认为 0, 即每种系统只创建一个实例
    - `--dup-each=1` 指每种系统额外多创建一个实例, 即每种系统创建两个实例
    - 指定为其它数据类同, 都是"增加多少倍"的含义
- 新创建的 ENV 会有连接失败的情况, 因部分系统启动较慢, 等一分钟再试
- 执行非常耗时的命令时, 建议使用`nohup $CMD >/tmp/log &`的形式启动, 而后通过查看日志获得执行结果

##### 示例

> ```shell
> # 短参数风格
> # 'centos7' 指 CentOS 7.x 全系列
> tt env add [-M 1024] [-C 8] -s centos7 [-p 80 -p 443] [-l 3600] [-d 1] ENV_NAME
>
> # 长参数风格,
> # 指定的端口会被影射为外部可访问的有效端口
> tt env add [--mem-size=1024] [--cpu-num=8] --os-prefix=centos7.3,ubuntu18.04 [--vm-port=80,443] [--dup-each=2] ENV_NAME
> ```

#### 删除环境

##### 命令参数

> ```shell
> tt-env-del
>
> USAGE:
>     tt env del [ENV]...
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

注意:

- 环境运行期间产生的所有数据将会丢失
- 若想保留数据, 请使用 `tt env stop`

##### 示例

> ```shell
> # 删除指定的一个或多个环境
> tt env del ENV_1 ENV_2 ENV_3
> ```

#### 停止环境

停止所有 VM 进程, 但保留所有的数据以备下次重启.

##### 命令参数

> ```shell
> tt-env-stop
>
> USAGE:
>     tt env stop <ENV>...
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

#### 启动环境

> ```shell
> tt-env-start
>
> USAGE:
>     tt env start <ENV>...
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

#### 查看环境属性

##### 命令参数

> ```shell
> tt-env-list
>
> 显示自己创建的所有环境列表
>
> USAGE:
>     tt env list
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
> ```

##### 命令参数

> ```shell
> tt-env-listall
>
> 显示全局创建的所有环境列表, 包括他人创建的
>
> USAGE:
>     tt env listall
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
> ```

##### 命令参数

> ```shell
> tt-env-show
>
> 查看指定环境的详细信息: 主机列表、生命周期等
>
> USAGE:
>     tt env show [ENV]...
>
> FLAGS:
>     -h, --help       Prints help information
>     -V, --version    Prints version information
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

#### 修改环境属性

##### 命令参数

> ```shell
> tt-env-update
>
> USAGE:
>     tt env update [FLAGS] [OPTIONS] <ENV>...
>
> FLAGS:
>         --SSSS              指定任意生命周期.
>     -y, --allow-outgoing    允许虚拟机对外连网.
>     -n, --deny-outgoing     禁止虚拟机对外连网.
>     -h, --help              Prints help information
>         --kick-dead         清除所有失去响应的 VM 实例.
>     -V, --version           Prints version information
>
> OPTIONS:
>     -C, --cpu-num <CPU_SIZE>        新的 CPU 数量.
>         --kick-os <OS_PREFIX>... 待剔除的系统名称前缀.
>         --kick-vm <VM_ID>... 待剔除的 VM 的 ID.
>     -l, --life-time <TIME>          新的生命周期.
>     -M, --mem-size <MEM_SIZE>       新的内存容量, 单位: MB.
>     -p, --vm-port <PORT>... 新的网络端口集合(全量替换, 非增量计算).
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

##### 示例

> ```shell
> # 生命周期默认 1 小时, 最长 6 个小时
> tt env update --life-time=$[6 * 3600] ENV_1 ENV_2
>
> # 排除指定的系统
> tt env update --kick-vm=fire:centos,qemu:ubuntu ENV_1 ENV_2
> ```

#### 往 VM 中布署文件(如: 推送产品包)

##### 命令参数

> ```shell
> tt-env-push
>
> 将产品包布署到指定一个或多个环境中, 上传的文件位于 /tmp/ 目录下
>
> USAGE:
>     tt env push [FLAGS] [OPTIONS] <ENV>...
>
> FLAGS:
>     -h, --help       Prints help information
>         --use-ssh    使用 SSH 协议通信.
>     -V, --version    Prints version information
>
> OPTIONS:
>     -f, --file-path <PATH>     文件在本地的路径.
>     -s, --os-prefix <OS>... 按系统名称前缀筛选.
>     -t, --time-out <TIME>      可执行的最长时间, 单位: 秒.
>     -m, --vm-id <VM>... 按 VmId 精确筛选.
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

#### 从 VM 中下载文件

##### 命令参数

> ```shell
> tt-env-get
>
> 从一个或多个环境中下载指定的文件
>
> USAGE:
>     tt env get [FLAGS] [OPTIONS] <ENV>...
>
> FLAGS:
>     -h, --help       Prints help information
>         --use-ssh    使用 SSH 协议通信.
>     -V, --version    Prints version information
>
> OPTIONS:
>     -f, --file-path <PATH>     文件在远程的路径.
>     -s, --os-prefix <OS>... 按系统名称前缀筛选.
>     -t, --time-out <TIME>      可执行的最长时间, 单位: 秒.
>     -m, --vm-id <VM>... 按 VmId 精确筛选.
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

#### 执行命令

##### 命令参数

> ```shell
> tt-env-run
>
> 向指定环境下的主机批量执行相同的操作
>
> USAGE:
>     tt env run [FLAGS] [OPTIONS] <ENV>...
>
> FLAGS:
>     -x, --config-spy     注册到 SPY 监控系统.
>     -h, --help           Prints help information
>     -i, --interactive    交互式串行操作.
>         --use-ssh        使用 SSH 协议通信.
>     -V, --version        Prints version information
>
> OPTIONS:
>     -c, --cmd <CMD>            SHELL 命令.
>     -s, --os-prefix <OS>... 按系统名称前缀筛选.
>     -f, --script <PATH>        脚本文件的本地路径.
>     -t, --time-out <TIME>      可执行的最长时间, 单位: 秒.
>     -m, --vm-id <VM>... 按 VmId 精确筛选.
>
> ARGS:
>     <ENV>... 一个或多个环境名称.
> ```

##### 示例

> ```shell
> # 执行一些简短的命令,
> # 其中不要使用引号, 容易出现解析错误,
> # 复杂的逻辑请写在脚本中, 然后使用'--script'去执行
> tt env run --cmd=<简短的命令> ENV_1 ENV_2 ENV_3
>
> # 运行脚本程序
> tt env run --script=<脚本程序的本地路径> ENV_1 ENV_2
>
> # 交互式串行执行命令, 如: 'passwd' 'ssh-keygen' 等需要按提示操作的命令
> tt env run --interactive ENV_1 ENV_2
> ```

#### 查看系统状态

##### 命令参数

> ```shell
> tt-status
>
> USAGE:
>     tt status [FLAGS]
>
> FLAGS:
>     -c, --client     查看客户端状态.
>     -h, --help       Prints help information
>     -s, --server     查看服务端状态.
>     -V, --version    Prints version information
> ```

##### 查看客户端本地概况

> ```shell
> (git)-[master]-% tt status
>
> [src/client/src/cfg_file.rs:31] self = Cfg {
>     server_list: Server {
>         addr: "10.0.9.22",
>         port: 9528,
>     },
>     client_id: "FanHui@1602312218@34366@fh",
> }
> ```

##### 查看服务端资源概况

> ```
> (git)-[master]-% tt status -s
>
> [src/client/src/ops/status.rs:27] r = {
>     "10.0.9.22:9527": RespGetServerInfo {
>         vm_total: 0,
>         cpu_total: 200,
>         cpu_used: 0,
>         mem_total: 40960,
>         mem_used: 0,
>         disk_total: 81920000,
>         disk_used: 0,
>         supported_list: [
>             "fire:centos-8.x:4.18.0-193.1.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.14.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.19.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.6.3.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.1.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.11.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.11.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.4.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.7.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.7.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-80.el8.x86_64",
>             "fire:dev:4.19.148.x86_64",
>         ],
>     },
>     "10.0.9.81:9527": RespGetServerInfo {
>         vm_total: 2,
>         cpu_total: 1000,
>         cpu_used: 16,
>         mem_total: 71680,
>         mem_used: 28384,
>         disk_total: 819200000,
>         disk_used: 81920,
>         supported_list: [
>             "fire:centos-8.x:4.18.0-147.0.3.el8.x86_64",
>             "fire:centos-8.x:4.18.0-147.3.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-147.5.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-147.8.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-147.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.1.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.14.2.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.19.1.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.6.3.el8.x86_64",
>             "fire:centos-8.x:4.18.0-193.el8.x86_64",
>             "qemu:centos-7.1:default",
>             "qemu:centos-7.2:default",
>             "qemu:centos-7.3:default",
>             "qemu:centos-7.4:default",
>             "qemu:centos-7.5:default",
>             "qemu:centos-7.6:default",
>             "qemu:centos-7.7:default",
>             "qemu:ubuntu-1604:default",
>             "qemu:ubuntu-1804:default",
>         ],
>     },
>     "10.0.9.207:9527": RespGetServerInfo {
>         vm_total: 1,
>         cpu_total: 400,
>         cpu_used: 12,
>         mem_total: 20480,
>         mem_used: 16000,
>         disk_total: 81920000,
>         disk_used: 40960,
>         supported_list: [
>             "fire:centos-7.x:3.10.0-1062.el7.x86_64",
>             "fire:centos-7.x:3.10.0-1127.el7.x86_64",
>             "fire:centos-7.x:3.10.0-693.el7.x86_64",
>             "fire:centos-7.x:3.10.0-862.el7.x86_64",
>             "fire:centos-7.x:3.10.0-957.el7.x86_64",
>             "fire:centos-7.x:4.14.0-115.el7.0.1.x86_64",
>             "qemu:alpine-3.12:default",
>             "qemu:centos-6.10:default",
>             "qemu:centos-7.0:default",
>             "qemu:centos-7.8:default",
>             "qemu:centos-8.2:default",
>             "qemu:freebsd-12.1:default",
>             "qemu:ubuntu-1404:default",
>             "qemu:ubuntu-2004:default",
>         ],
>     },
> }
> ```
