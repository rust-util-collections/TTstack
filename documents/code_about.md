# tt

tt 主要代码使用 Rust 编写.

Rust 是一门风格**紧凑**、运行**高效**的现代开发语言, 以**最少的代码量**实现**最高的性能**, 打破了几十年来的旷世难题: 静态语言与动态语言两者的优势不能兼得.

## 项目结构

```
$ (git)-[master]-% tree -F -I 'target' -L 1
.
├── Cargo.toml           # 项目配置文件
├── README.md            # 项目主文档
├── client/              # 客户端 tt 命令的代码实现
├── core/                # 服务端的核心逻辑实现
├── core_def/            # 从 core 模块中提取出的通用定义, 供 server 模块使用
├── documents/           # 项目详细文档
├── proxy/               # 分布式架构后端, 负责统筹调度多个 Server 的资源
├── rexec/               # 一个轻量级的"远程命令执行和文件转输"方案
├── server/              # 后端 Server 的代码实现, 可独立运行, 也可挂靠在 Proxy 之后
├── server_def/          # 从 server 模块中提取出的通用定义, 供 proxy 模块使用
├── ... # 部分文件没有显示...
└── tools/               # 外围脚本工具
```

## 代码规模

```
$ (git)-[master]-% find . -type f | grep -Ev 'target|\.(git|lock)' | xargs wc -l | grep -Ev '^ +[0-9]{1,2} '

   253 ./client/src/ops/env/run/mod.rs
   118 ./client/src/ops/env/run/ssh.rs
   156 ./client/src/ops/env/update.rs
   137 ./client/src/ops/mod.rs
   147 ./client/src/cfg_file.rs
   553 ./client/src/cmd_line.rs
   113 ./core/src/freebsd/nat/mod.rs
   179 ./core/src/freebsd/vm/mod.rs
   157 ./core/src/freebsd/mod.rs
   273 ./core/src/linux/nat/mod.rs
   245 ./core/src/linux/vm/engine/firecracker/suitable_env.rs
   372 ./core/src/linux/vm/engine/qemu.rs
   150 ./core/src/linux/vm/cgroup.rs
   229 ./core/src/linux/mod.rs
   970 ./core/src/def.rs
   164 ./core_def/src/lib.rs
   389 ./documents/user_guide.md
   418 ./proxy/src/hdr/mod.rs
   175 ./proxy/src/hdr/add_env.rs
   101 ./proxy/src/def.rs
   181 ./proxy/src/lib.rs
   146 ./proxy/tests/env/mod.rs
   282 ./proxy/tests/knead/mod.rs
   179 ./proxy/tests/standalone/mod.rs
   117 ./rexec/src/bin/cli.rs
   149 ./rexec/src/client.rs
   164 ./rexec/src/common.rs
   272 ./rexec/src/server.rs
   169 ./rexec/tests/integration.rs
   367 ./server/src/hdr/mod.rs
   103 ./server/tests/env/mod.rs
   241 ./server/tests/knead/mod.rs
   163 ./server/tests/standalone/mod.rs
   184 ./server_def/src/lib.rs
   159 ./README.md
 11547 total
```
