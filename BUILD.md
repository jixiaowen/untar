# Build and Deployment Guide

## Target Platform

This application is built and tested for **RedHat Enterprise Linux 7 (RHEL 7)** and compatible distributions (CentOS 7, Oracle Linux 7).

## System Requirements

### Minimum Requirements
- **OS**: RedHat 7 / CentOS 7 / Oracle Linux 7
- **glibc**: 2.17 or higher
- **RAM**: 2GB minimum, 4GB recommended
- **Disk**: 100MB for binary, additional space for temporary files

### Required Libraries
- `libssl` (OpenSSL)
- `libcrypto` (OpenSSL)
- Standard C library (glibc)

## Building from Source

### Method 1: Using Docker (Recommended for RedHat 7 Compatibility)

This is the easiest method to build binaries compatible with RedHat 7:

```bash
# Clone repository
git clone <repository-url>
cd untar

# Build using Docker
docker build -f Dockerfile.build -t untar-build:centos7 .

# Build x86_64 binary
docker run --rm -v $PWD:/build untar-build:centos7 cargo build --release

# Build aarch64 binary (cross-compilation)
docker run --rm \
  -v $PWD:/build \
  -e CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  -e CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
  untar-build:centos7 \
  bash -c "yum install -y gcc-aarch64-linux-gnu && rustup target add aarch64-unknown-linux-gnu && cargo build --release --target aarch64-unknown-linux-gnu"

# Binary locations
./target/release/untar                    # x86_64
./target/aarch64-unknown-linux-gnu/release/untar  # aarch64
```

### Method 2: Native Build on RedHat 7

#### Prerequisites

```bash
# Install build tools
sudo yum install -y gcc gcc-c++ make git

# Install OpenSSL development libraries
sudo yum install -y openssl-devel

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source $HOME/.cargo/env
```

#### Build for x86_64

```bash
# Clone repository
git clone <repository-url>
cd untar

# Build release binary
cargo build --release

# Binary location
./target/release/untar
```

#### Cross-compile for aarch64

```bash
# Install cross-compilation tools
sudo yum install -y gcc-aarch64-linux-gnu

# Add aarch64 target
rustup target add aarch64-unknown-linux-gnu

# Build for aarch64
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
cargo build --release --target aarch64-unknown-linux-gnu

# Binary location
./target/aarch64-unknown-linux-gnu/release/untar
```

## Deployment

### Deploy to RedHat 7 Server

1. **Transfer binary to server**:
   ```bash
   scp ./target/release/untar user@server:/path/to/deployment/
   ```

2. **Set executable permissions**:
   ```bash
   ssh user@server
   chmod +x /path/to/deployment/untar
   ```

3. **Verify dependencies**:
   ```bash
   # Check OpenSSL libraries
   ldd /path/to/deployment/untar | grep ssl
   
   # Should show libssl and libcrypto
   ```

4. **Run the application**:
   ```bash
   /path/to/deployment/untar --tar archive.tar --xml manifest.xml --dst /hdfs/path
   ```

### Troubleshooting

#### Missing OpenSSL Libraries

If you encounter errors about missing OpenSSL libraries:

```bash
# Check if OpenSSL is installed
rpm -qa | grep openssl

# Install OpenSSL if missing
sudo yum install -y openssl openssl-devel

# Find library location
ldconfig -p | grep libssl
```

#### GLIBC Version Mismatch

If you see "GLIBC_2.28 not found" or similar errors:

```bash
# Check glibc version
ldd --version

# For RedHat 7, glibc should be 2.17
# If using a different distribution, you may need to rebuild
```

#### Core Dump Analysis

If the application crashes with a core dump:

```bash
# Enable core dumps
ulimit -c unlimited

# Run with backtrace
RUST_BACKTRACE=1 ./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path

# Analyze core dump (if generated)
gdb ./untar core-tokio-*
(gdb) bt full
```

## GitHub Actions CI/CD

The project includes automated builds for RedHat 7 compatibility using Docker:

### Build Process

- **Runner Environment**: GitHub Actions ubuntu-latest
- **Build Environment**: CentOS 7 Docker container (ensures glibc 2.17 compatibility)
- **Targets**: x86_64 and aarch64
- **Artifacts**: Pre-built binaries available in GitHub releases

### How It Works

The workflow uses Docker to build binaries in a CentOS 7 environment, ensuring:

1. **glibc 2.17 compatibility** - Same as RedHat 7
2. **Consistent build environment** - Reproducible builds
3. **Cross-platform support** - Both x86_64 and aarch64

### Manual Trigger

You can manually trigger the build workflow:
1. Go to Actions tab in GitHub
2. Select "Build and Release" workflow
3. Click "Run workflow" button

### Local Docker Build

You can replicate the CI/CD build locally:

```bash
# Build the Docker image
docker build -f Dockerfile.build -t untar-build:centos7 .

# Build the binary (same as CI/CD)
docker run --rm -v $PWD:/build untar-build:centos7 cargo build --release
```

## Environment Variables

### Optional Environment Variables

```bash
# Enable detailed logging
RUST_LOG=debug ./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path

# Enable backtrace on panic
RUST_BACKTRACE=1 ./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path

# Set Hadoop configuration directory
export HADOOP_CONF_DIR=/etc/hadoop/conf

# Set Kerberos ticket cache (if using Kerberos)
export KRB5CCNAME=/tmp/krb5cc_$(id -u)
```

## Performance Tuning

### Thread Count

Adjust the number of parallel workers based on your system resources:

```bash
# Default: 10 threads
./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path

# Reduce threads for memory-constrained systems
./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path --threads 2

# Increase threads for high-performance systems
./untar --tar archive.tar --xml manifest.xml --dst /hdfs/path --threads 20
```

### Memory Considerations

Each thread uses approximately 64KB buffer for decompression. Calculate memory usage:

```
Total Memory ≈ Threads × 64KB + Base Overhead (~10MB)
```

Example with 10 threads: ~10.6MB

## Security Notes

### Kerberos Authentication

For HDFS clusters using Kerberos:

1. Ensure `libgssapi_krb5` is installed:
   ```bash
   sudo yum install -y krb5-devel
   ```

2. Obtain a ticket before running:
   ```bash
   kinit username@REALM
   ```

3. Verify ticket:
   ```bash
   klist
   ```

### File Permissions

The application requires:
- Read access to the TAR file
- Read access to the XML manifest file
- Write access to the target HDFS path

## Support

For issues or questions:
1. Check the troubleshooting section above
2. Review logs with `RUST_LOG=debug`
3. Analyze core dumps if available
4. Check system resources and dependencies
