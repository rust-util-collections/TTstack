//!
//! # NAT
//!
//! 只处理必要的 NAT 逻辑, 不要配置过滤规则, 那是系统管理员的工作.
//!

pub(crate) use real::*;

#[cfg(not(feature = "nft"))]
pub(crate) mod real {
    //! 使用 Qemu 的 hostfwd 实现 nat 转发,
    //! 此处只需实现空接口兼容上层逻辑即可.

    use crate::Vm;
    use myutil::err::*;

    #[inline(always)]
    pub(in crate::linux) fn init(_serv_ip: &str) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn set_rule(_vm: &Vm) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn clean_rule(_vm_set: &[&Vm]) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn deny_outgoing(_vm_set: &[&Vm]) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn allow_outgoing(_vm_set: &[&Vm]) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "nft")]
pub(crate) mod real {
    //! # NAT
    //!
    //! #### 创建 map, 用于 dnat 影射
    //!
    //! ```shell
    //! nft '
    //!     add map TABLE PORT_TO_PORT { type inet_service: inet_service; };
    //!     add map TABLE PORT_TO_IPV4 { type inet_service: ipv4_addr; };
    //! '
    //! ```
    //!
    //! #### 基于 map 设定 dnat 规则
    //! ```shell
    //! nft 'add rule ip TABLE CHAIN dnat tcp dport map @PORT_TO_IPV4: tcp dport map @PORT_TO_PORT'
    //! ```
    //!
    //! #### 通过增删 map 实现动态路由
    //! ```shell
    //! nft '
    //!     add element TABLE PORT_TO_IPV4 { 8080 : 10.10.10.10 };
    //!     add element TABLE PORT_TO_IPV4 { 9999 : 1.1.1.1 };
    //!     add element TABLE PORT_TO_PORT { 8080 : 80 };
    //!     delete element TABLE PORT_TO_IPV4 { 8080, 9999 };
    //! '
    //! ```
    //!
    //! # FILTER
    //!
    //! #### 创建 set, 用于禁止特定的 ENV 主动对外连网
    //!
    //! ```shell
    //! nft 'add set ip TABLE BLACK_LIST { type ipv4_addr; };'
    //! ```
    //!
    //! #### 基于 set 设定 filter 规则
    //!
    //! ```shell
    //! nft 'add rule ip TABLE CHAIN ip saddr @BLACK_LIST drop'
    //! ```
    //!
    //! #### 通过增删 set 实现动态白名单
    //!
    //! ```shell
    //! nft '
    //!     add element TABLE BLACK_LIST { 10.10.10.10, 10.10.10.11 };
    //!     delete element TABLE BLACK_LIST { 10.10.10.10, 10.10.10.11 };
    //! '
    //! ```

    use crate::{asleep, Vm, POOL};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};
    use parking_lot::Mutex;
    use std::{collections::HashSet, mem, process, sync::Arc};

    const TABLE_PROTO: &str = "ip";
    const TABLE_NAME: &str = "tt-core";

    lazy_static! {
        static ref RULE_SET: Arc<Mutex<Vec<String>>> =
            Arc::new(Mutex::new(vct![]));
        static ref RULE_SET_ALLOW_FAIL: Arc<Mutex<Vec<String>>> =
            Arc::new(Mutex::new(vct![]));
    }

    // nft 初始化
    pub(in crate::linux) fn init(serv_ip: &str) -> Result<()> {
        set_rule_cron();

        let arg = format!("
            add table {proto} {table};
            delete table {proto} {table};
            add table {proto} {table};

            add set {proto} {table} BLACK_LIST {{ type ipv4_addr; }};
            add chain {proto} {table} FWD_CHAIN {{ type filter hook forward priority 0; policy accept; }};
            add rule {proto} {table} FWD_CHAIN ct state established,related accept;
            add rule {proto} {table} FWD_CHAIN {proto} saddr @BLACK_LIST drop;

            add map {proto} {table} PORT_TO_PORT {{ type inet_service: inet_service; }};
            add map {proto} {table} PORT_TO_IPV4 {{ type inet_service: ipv4_addr; }};
            add chain {proto} {table} DNAT_CHAIN {{ type nat hook prerouting priority -100; }};
            add chain {proto} {table} SNAT_CHAIN {{ type nat hook postrouting priority 100; }};
            add rule {proto} {table} DNAT_CHAIN dnat tcp dport map @PORT_TO_IPV4: tcp dport map @PORT_TO_PORT;
            add rule {proto} {table} DNAT_CHAIN dnat udp dport map @PORT_TO_IPV4: udp dport map @PORT_TO_PORT;
            add rule {proto} {table} SNAT_CHAIN ip saddr 10.0.0.0/8 ip daddr != 10.0.0.0/8 snat to {pubip};
            ",
            proto=TABLE_PROTO,
            table=TABLE_NAME,
            pubip=serv_ip,
        );

        nft_exec(&arg).c(d!())
    }

    // 添加新的规则集
    pub(crate) fn set_rule(vm: &Vm) -> Result<()> {
        if vm.port_map.is_empty() {
            return Ok(());
        }

        let mut port_to_ipv4 = vct![];
        let mut port_to_port = vct![];

        vm.port_map.iter().for_each(|(vmport, pubport)| {
            port_to_ipv4.push(format!("{}:{}", pubport, vm.ip.as_str()));
            port_to_port.push(format!("{}:{}", pubport, vmport));
        });

        let arg = format!(
            "
            add element {proto} {table} PORT_TO_IPV4 {{ {ptoip} }};
            add element {proto} {table} PORT_TO_PORT {{ {ptop} }};
            ",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ptoip = port_to_ipv4.join(","),
            ptop = port_to_port.join(","),
        );

        RULE_SET.lock().push(arg);

        Ok(())
    }

    // 清理指定端口对应的 NAT 规则
    pub(crate) fn clean_rule(vm_set: &[&Vm]) -> Result<()> {
        // Ports are almost always duplicated,
        // use `HashSet` to keep unique
        let port_set = vm_set
            .iter()
            .map(|vm| vm.port_map.values())
            .flatten()
            .collect::<HashSet<_>>();

        // Do NOT work for empty
        if port_set.is_empty() {
            return Ok(());
        }

        let arg = format!(
            "
            delete element {proto} {table} PORT_TO_IPV4 {{ {pub_port} }};
            delete element {proto} {table} PORT_TO_PORT {{ {pub_port} }};
            ",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            pub_port = port_set
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );

        RULE_SET.lock().push(arg);

        // 解锁已释放的 VM 地址
        omit!(allow_outgoing(vm_set));

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn deny_outgoing(vm_set: &[&Vm]) -> Result<()> {
        let ip_set = vm_set
            .iter()
            .map(|vm| vm.ip.to_string())
            .collect::<Vec<_>>();
        if ip_set.is_empty() {
            return Ok(());
        }

        let arg = format!(
            "add element {proto} {table} BLACK_LIST {{ {ip_set} }};",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ip_set = ip_set.join(","),
        );

        RULE_SET.lock().push(arg);

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn allow_outgoing(vm_set: &[&Vm]) -> Result<()> {
        let ip_set = vm_set
            .iter()
            .map(|vm| vm.ip.to_string())
            .collect::<Vec<_>>();
        if ip_set.is_empty() {
            return Ok(());
        }

        let arg = format!(
            "delete element {proto} {table} BLACK_LIST {{ {ip_set} }};",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ip_set = ip_set.join(","),
        );

        RULE_SET_ALLOW_FAIL.lock().push(arg);

        Ok(())
    }

    // 执行 nft 命令
    #[inline(always)]
    fn nft_exec(arg: &str) -> Result<()> {
        let arg = format!("nft '{}'", arg);
        let res = process::Command::new("sh")
            .args(&["-c", &arg])
            .output()
            .c(d!())?;
        if res.status.success() {
            Ok(())
        } else {
            Err(eg!(String::from_utf8_lossy(&res.stderr)))
        }
    }

    // nft 并发设置规则会概率性失败,
    // 以异步模式延时 1 秒应用收集到的所有规则
    fn set_rule_cron() {
        POOL.spawn_ok(async {
            loop {
                asleep(2).await;
                let args = mem::take(&mut *RULE_SET.lock());
                if !args.is_empty() {
                    POOL.spawn_ok(async move {
                        asleep(1).await;
                        info_omit!(nft_exec(dbg!(&args.join(""))));
                    });
                }

                let args_allow_fail =
                    mem::take(&mut *RULE_SET_ALLOW_FAIL.lock());
                if !args_allow_fail.is_empty() {
                    POOL.spawn_ok(async move {
                        asleep(1).await;
                        args_allow_fail.iter().for_each(|arg| {
                            info_omit!(nft_exec(dbg!(&arg)));
                        })
                    });
                }
            }
        });
    }
}
