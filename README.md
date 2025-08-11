# TTStack

![](https://tokei.rs/b1/github/rustcc/ttstack)
![pipeline status](https://gitlab.com/ktmlm/ttstack/badges/master/pipeline.svg)
![coverage report](https://gitlab.com/ktmlm/ttstack/badges/master/coverage.svg)

A light-weight 'private cloud solution' for SMEs (Small and Medium Enterprises), bringing significant help and commercial value to start-up companies.

A lightweight private cloud platform for small and medium enterprises that can rapidly generate various virtual machine environments, providing efficient foundational infrastructure for product compatibility verification and automated testing scenarios.

> #### The Efficiency Paradox of SMEs
>
> Start-up SMEs typically have weak technical capabilities, with many production methods still stuck in primitive manual processes.
>
> **This creates a paradox:**
>
> > Theoretically, start-ups need to win on efficiency to surpass large companies. Indeed, in terms of "human" subjective efficiency, such as "management processes," most start-ups can achieve this due to their simple business scenarios. However, in terms of "technical processes," they lag behind dramatically. The result is naturally disastrous, like Polish cavalry in WWII charging at German tanks with raised sabers (primitive tools) - no matter how swift the horsemen ("human" efficiency), they cannot overcome German soldiers sitting in tanks ("tool" efficiency).
> >
> > Meanwhile, due to start-ups' funding shortages, few third-party companies are willing to explore this market (unprofitable). Start-ups themselves lack sufficient resources to solve this problem, creating a vicious cycle of perpetual "firefighting" low-efficiency states until the company fails or a talented individual single-handedly changes the situation.
>
> This project aims for simplicity and ease of use, striving to solve this "neglected" paradox:
>
> - Specifically designed for SMEs with distributed, scalable, and extensible architecture
> - Fully utilizing hardware resources: unified scheduling of all hardware resources through cloud platform, significantly improving resource utilization and flexibility
> - Extremely low system setup and maintenance costs: operations staff typically need only 30 minutes to set up a complete TT private cloud platform
> - Extremely low learning and usage costs: end users typically need only 10 minutes to proficiently use TT client to create required virtual environments
> - **Cost-effective, yes! Very cost-effective!** You don't need to spend huge amounts maintaining a dedicated cloud team (OpenStack/K8S professionals are usually very expensive)
> - Are public clouds really cheap? Convenient? Secure? Those who have used them know the answer
> - ...

## Main Use Cases

1. Extensive platform compatibility verification
    - Arbitrary cross-combinations in two directions:
        1. Various OS categories and versions: Linux, BSD, Windows, macOS, etc.
        2. Various hardware platforms: AMD64, X86, AArch64, ARM, MIPS, RISC-V, SPARC, etc.
2. Integration with DevOps systems for automated CI/CD functionality
3. Use as native compilation platform
    - Directly provision full native OS environments, avoiding cross-compilation complexity and potential issues
4. Use as short-term or long-term debugging environment
    - TT can be viewed as a traditional cloud platform, provisioning VMs for development and testing
5. Others...

## Technical Features

- Clean and efficient resource management
    - Each VM exists in an independent Cgroup with accurate and error-free resource cleanup
    - [Optional] Use FireCracker to rapidly create large numbers of lightweight MicroVMs
    - [Default] Use ZFS `snapshot/clone` mechanisms for VMs to achieve native IO performance
    - [Default] Use nftables `SET/MAP` advanced data structures to manage network ports
    - Service processes run in separate `PID NS`, automatically destroying all resources on service exit
    - Automatic VM lifecycle management through `Rust Drop` mechanisms
    - ...
- Distributed scalable architecture
    - Backend supports multi-machine distributed architecture, completely transparent to users
- Lightweight communication model
    - Client/Server communication based on UDP/SCTP
    - Self-developed remote command execution tool, far more efficient than SSH protocol
- Image source decoupled from services
    - Supported system images can be added anytime without server downtime
    - Supports multiple VM engines: Qemu, FireCracker, Bhyve, etc.
    - VM type identified by image name prefix, e.g., fire:centos-7.3:3.10.e17.x86_64
- Developed in `Rust` language
    - Safe and stable
    - High-performance execution
    - Complete documentation
    - Native cross-platform
    - ...

## Quick Start

#### Compilation

```shell
make install
export PATH=~/.cargo/bin:$PATH
```

#### Starting the Server

> **Note**
>
> Image files must **NOT** be stored in '/tmp' or its subdirectories, as this will prevent image information scanning (ttserver's '/tmp' path is private and isolated from the external environment).

```shell
# Slave Server 1
ttserver \
        --image-path /home/images \
        --cfgdb-path /tmp \
        --cpu-total 2 \
        --mem-total $[4 * 1024 * 1024] \
        --disk-total $[40 * 1024 * 1024] \
        --serv-addr 127.0.0.1 \
        --serv-port 20000

# Slave Server 2
ttserver \
        --image-path /home/images \
        --cfgdb-path /tmp \
        --cpu-total 2 \
        --mem-total $[4 * 1024 * 1024] \
        --disk-total $[40 * 1024 * 1024] \
        --serv-addr 127.0.0.1 \
        --serv-port 20001

# Proxy - distributed proxy service responsible for scheduling Slave Server resources
ttproxy \
        --proxy-addr 127.0.0.1:20002 \
        --server-set 127.0.0.1:20000,127.0.0.1:20001
```

#### Client Operations

> **Tips**
> - Complete client operation documentation: [User Guide](./documents/user_guide.md)
> - The "/home/images" path needs to contain properly bootable Qemu image files
> - The "/etc/rc.local" file in image files needs to be replaced with the project's customized "[rc.local](./tools/images/linux_vm/rc.local)"

```shell
# Configure server address
# Can be either Proxy address or individual Slave Server addresses
# Here we configure the Proxy address to demonstrate distributed architecture scheduling
tt config --serv-addr 127.0.0.1 --serv-port 20002

# View local client information
tt status

# View server resource information
tt status --server

# Create an "ENV"
# The basic management unit in TT is ENV (a collection of VMs)
# Created VM types are matched by system prefix, case-insensitive
# For example:
#     - cent will match all CentOS systems
#     - ubuntu2004 will only match Ubuntu2004 system
tt env add TEST --os-prefix=cent,ubuntu2004

# View list of created ENVs
tt env list

# View details of a specific created ENV
tt env show TEST

# Execute the same command on all VMs in ENV
tt env run TEST --use-ssh --cmd 'ls /'

# Delete ENV
# All VMs and their related data will be cleaned up
tt env del TEST
```

## Detailed Documentation

- [Development Roadmap](./documents/roadmap.md)
- [End User Guide](./documents/user_guide.md)
- [System Administration Guide](./documents/system_admin.md)
- [Architecture Design and Technology Selection](./documents/arch_design.md)
- [Project Structure and Code Scale](./documents/code_about.md)

> #### API Documentation
>
> ```shell
> # Execute in Rust development environment
> make doc
> ```

## Known Issues

- Single ENV with 400+ VMs may encounter exceptions
    - Cause: Large ENVs exceed UDP single communication maximum payload
    - Currently mitigated through data compression, future migration to HTTP/SCTP planned

## Statistics

```
# Tips: cargo install tokei

(git)-[master]-% tokei
===============================================================================
 Language            Files        Lines         Code     Comments       Blanks
===============================================================================
 BASH                    1            5            2            1            2
 Makefile                1          108           92            0           16
 Shell                   8          278          198           30           50
 TOML                   10          234          192            1           41
-------------------------------------------------------------------------------
 Markdown               10          662            0          492          170
 |- Shell                4          372          333           23           16
 (Total)                           1034          333          515          186
-------------------------------------------------------------------------------
 Rust                   77         9182         7770          287         1125
 |- Markdown            71          803           41          707           55
 (Total)                           9985         7811          994         1180
===============================================================================
 Total                 107        11644         8628         1541         1475
===============================================================================
```