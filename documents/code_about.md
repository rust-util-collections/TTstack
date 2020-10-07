# tt

tt 主要代码使用 Rust 编写.

Rust 是一门风格**紧凑**、运行**高效**的现代开发语言, 以**最少的代码量**实现**最高的性能**, 打破了几十年来的旷世难题: 静态语言与动态语言两者的优势不能兼得.

## 项目结构

```
$ (git)-[master]-% tree -F -I 'target' -L 1
.
├── Cargo.toml               # 项目配置文件
├── README.md                # 项目主文档
├── documents/               # 项目详细文档
├── src/core/                # 服务端的核心逻辑实现
├── src/core_def/            # 从 core 模块中提取出的通用定义, 供 server 模块使用
├── src/server/              # 后端 Server 的代码实现, 可独立运行, 也可挂靠在 Proxy 之后
├── src/server_def/          # 从 server 模块中提取出的通用定义, 供 proxy 模块使用
├── src/proxy/               # 分布式架构后端, 负责统筹调度多个 Server 的资源
├── src/rexec/               # 一个轻量级的"远程命令执行和文件转输"方案
├── src/client/              # 客户端 tt 命令的代码实现
├── tools/                   # 外围脚本工具
└── ...
```

## 代码规模

```
$ (git)-[master]-% find . -type f \
    | grep -Ev 'target|tools/(.*kernel_config|firecracker)|\.(git|lock)' \
    | xargs wc -l \
    | grep -Ev '^ +[0-9]{1,2} '

   367 ./src/server/src/hdr/mod.rs
   103 ./src/server/tests/env/mod.rs
   164 ./src/server/tests/standalone/mod.rs
   242 ./src/server/tests/knead/mod.rs
   184 ./src/server_def/src/lib.rs
   149 ./src/rexec/src/client.rs
   272 ./src/rexec/src/server.rs
   164 ./src/rexec/src/common.rs
   117 ./src/rexec/src/bin/cli.rs
   169 ./src/rexec/tests/integration.rs
   553 ./src/client/src/cmd_line.rs
   157 ./src/client/src/ops/env/update.rs
   118 ./src/client/src/ops/env/run/ssh.rs
   253 ./src/client/src/ops/env/run/mod.rs
   137 ./src/client/src/ops/mod.rs
   147 ./src/client/src/cfg_file.rs
   164 ./src/core_def/src/lib.rs
   977 ./src/core/src/def.rs
   113 ./src/core/src/freebsd/nat/mod.rs
   175 ./src/core/src/freebsd/vm/mod.rs
   137 ./src/core/src/freebsd/mod.rs
   275 ./src/core/src/linux/nat/mod.rs
   150 ./src/core/src/linux/vm/cgroup.rs
   372 ./src/core/src/linux/vm/engine/qemu.rs
   245 ./src/core/src/linux/vm/engine/firecracker/suitable_env.rs
   205 ./src/core/src/linux/mod.rs
   175 ./src/proxy/src/hdr/add_env.rs
   418 ./src/proxy/src/hdr/mod.rs
   101 ./src/proxy/src/def.rs
   183 ./src/proxy/src/lib.rs
   146 ./src/proxy/tests/env/mod.rs
   180 ./src/proxy/tests/standalone/mod.rs
   283 ./src/proxy/tests/knead/mod.rs
   389 ./documents/user_guide.md
   162 ./README.md
   107 ./Makefile
 11597 total
```
