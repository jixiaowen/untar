# Untar Tool

一个基于 Rust 的高性能解压工具，支持从 tar 包中解压文件并直接上传到 HDFS。

## 功能特性

- 支持根据日期和 XML 文件名查找目标路径
- 解析 XML 文件获取文件列表和大小信息
- 支持 gzip 和 Z (compress) 两种压缩格式
- 多线程并发解压，提高处理效率
- 解压后文件直接上传到 HDFS，不落本地磁盘
- 支持 Kerberos 认证
- 文件完整性校验（对比 XML 记录大小和实际大小）
- 内存控制机制，避免内存占用过高

## 编译和安装

### 从源码编译

```bash
# 克隆仓库
git clone <repository-url>
cd untar

# 编译
cargo build --release

# 编译后的二进制文件位于 target/release/untar
```

### 下载预编译二进制文件

从 GitHub Releases 页面下载对应架构的二进制文件：
- `untar-linux-x86_64` - Linux x86_64 架构
- `untar-linux-aarch64` - Linux ARM64 架构

下载后添加执行权限：
```bash
chmod +x untar-linux-x86_64
```

## 使用方法

### 基本用法

```bash
./untar \
  --date 2024-01-15 \
  --tar-name archive.tar.gz \
  --xml-name manifest.xml \
  --search-path /data/archives \
  --hdfs-path /user/hdfs/output
```

### 完整参数说明

```
USAGE:
    untar [OPTIONS]

OPTIONS:
    -d, --date <DATE>              日期字符串，用于查找目标路径
    -t, --tar-name <TAR_NAME>      tar 包文件名
    -x, --xml-name <XML_NAME>      XML 文件名
    -s, --search-path <SEARCH_PATH> 搜索路径
    -h, --hdfs-path <HDFS_PATH>    HDFS 目标路径
        --kerberos-principal <KERBEROS_PRINCIPAL> Kerberos principal（可选）
        --kerberos-keytab <KERBEROS_KEYTAB>       Kerberos keytab 文件路径（可选）
    -j, --threads <THREADS>        并发线程数 [default: 4]
        --max-memory-mb <MAX_MEMORY_MB> 最大内存使用量（MB）[default: 512]
```

### 使用 Kerberos 认证

```bash
./untar \
  --date 2024-01-15 \
  --tar-name archive.tar.gz \
  --xml-name manifest.xml \
  --search-path /data/archives \
  --hdfs-path /user/hdfs/output \
  --kerberos-principal user@EXAMPLE.COM \
  --kerberos-keytab /path/to/user.keytab
```

### 自定义并发数和内存限制

```bash
./untar \
  --date 2024-01-15 \
  --tar-name archive.tar.gz \
  --xml-name manifest.xml \
  --search-path /data/archives \
  --hdfs-path /user/hdfs/output \
  --threads 8 \
  --max-memory-mb 1024
```

## XML 文件格式

XML 文件应包含文件名和大小信息，格式如下：

```xml
<files>
    <file>
        <name>file1.txt</name>
        <size>1024</size>
    </file>
    <file>
        <name>file2.txt</name>
        <size>2048</size>
    </file>
</files>
```

## 工作流程

1. 根据日期和 XML 文件名在指定路径下搜索目标目录
2. 解析 XML 文件，提取文件列表和大小信息
3. 查找 tar 包文件
4. 连接到 HDFS（支持 Kerberos 认证）
5. 使用多线程并发解压 tar 包中的文件
6. 校验文件完整性（对比 XML 记录大小和实际大小）
7. 将解压后的文件直接上传到 HDFS
8. 输出处理结果和统计信息

## 开发

### 运行测试

```bash
cargo test
```

### 代码检查

```bash
cargo clippy
```

### 格式化代码

```bash
cargo fmt
```

## 许可证

[MIT License](LICENSE)
