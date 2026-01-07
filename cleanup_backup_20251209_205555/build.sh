#!/bin/bash
set -e

# Hybrid Blockchain Build Script
# Builds both C++ core engine and Rust system components

echo "🔨 Building Hybrid C++/Rust Blockchain System..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BUILD_TYPE=${BUILD_TYPE:-Release}
PARALLEL_JOBS=${PARALLEL_JOBS:-$(nproc)}
INSTALL_DEPS=${INSTALL_DEPS:-false}

print_step() {
    echo -e "${BLUE}==>${NC} ${YELLOW}$1${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check system requirements
check_requirements() {
    print_step "Checking system requirements..."
    
    # Check for required tools
    local missing_tools=()
    
    command -v cmake >/dev/null 2>&1 || missing_tools+=("cmake")
    command -v rustc >/dev/null 2>&1 || missing_tools+=("rust")
    command -v cargo >/dev/null 2>&1 || missing_tools+=("cargo")
    command -v pkg-config >/dev/null 2>&1 || missing_tools+=("pkg-config")
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        print_error "Missing required tools: ${missing_tools[*]}"
        echo "Please install the missing tools and run again."
        exit 1
    fi
    
    # Check for required libraries
    local missing_libs=()
    
    pkg-config --exists openssl || missing_libs+=("libssl-dev")
    pkg-config --exists libsecp256k1 || missing_libs+=("libsecp256k1-dev")
    
    if [ ${#missing_libs[@]} -ne 0 ]; then
        print_error "Missing required libraries: ${missing_libs[*]}"
        
        if [ "$INSTALL_DEPS" = "true" ]; then
            print_step "Installing missing dependencies..."
            if command -v apt >/dev/null 2>&1; then
                sudo apt update
                sudo apt install -y cmake build-essential pkg-config \
                    libssl-dev libsecp256k1-dev libleveldb-dev \
                    libboost-all-dev
            elif command -v yum >/dev/null 2>&1; then
                sudo yum install -y cmake gcc-c++ pkg-config \
                    openssl-devel libsecp256k1-devel leveldb-devel \
                    boost-devel
            else
                print_error "Cannot automatically install dependencies on this system"
                exit 1
            fi
        else
            echo "Run with INSTALL_DEPS=true to automatically install dependencies"
            exit 1
        fi
    fi
    
    print_success "All requirements satisfied"
}

# Build C++ core components
build_cpp_core() {
    print_step "Building C++ core engine..."
    
    cd cpp-core
    
    # Create build directory
    mkdir -p build
    cd build
    
    # Configure with CMake
    cmake .. \
        -DCMAKE_BUILD_TYPE=${BUILD_TYPE} \
        -DBUILD_TESTS=ON \
        -DBUILD_BENCHMARKS=ON
    
    # Build with parallel jobs
    make -j${PARALLEL_JOBS}
    
    # Run C++ tests
    print_step "Running C++ core tests..."
    if make test; then
        print_success "C++ core tests passed"
    else
        print_error "C++ core tests failed"
        exit 1
    fi
    
    cd ../..
    print_success "C++ core engine built successfully"
}

# Build Rust system components
build_rust_system() {
    print_step "Building Rust system components..."
    
    # Set Rust environment
    export RUSTFLAGS="-C target-cpu=native"
    
    # Build all Rust components
    if cargo build --release --all; then
        print_success "Rust system components built successfully"
    else
        print_error "Rust build failed"
        exit 1
    fi
    
    # Run Rust tests
    print_step "Running Rust system tests..."
    if cargo test --release --all; then
        print_success "Rust system tests passed"
    else
        print_error "Rust system tests failed"
        exit 1
    fi
}

# Run integration tests
run_integration_tests() {
    print_step "Running integration tests..."
    
    # Test FFI integration
    cargo test --release --package blockchain-ffi
    
    # Test end-to-end functionality
    if [ -d "integration-tests" ]; then
        cd integration-tests
        cargo test --release
        cd ..
    fi
    
    print_success "Integration tests passed"
}

# Generate documentation
generate_docs() {
    print_step "Generating documentation..."
    
    # Generate Rust documentation
    cargo doc --no-deps --document-private-items
    
    # Generate C++ documentation (if Doxygen is available)
    if command -v doxygen >/dev/null 2>&1; then
        cd cpp-core
        if [ -f "Doxyfile" ]; then
            doxygen
        fi
        cd ..
    fi
    
    print_success "Documentation generated"
}

# Install binaries
install_binaries() {
    print_step "Installing binaries..."
    
    # Install C++ libraries
    cd cpp-core/build
    sudo make install
    cd ../..
    
    # Install Rust binaries
    cargo install --path rust-system/blockchain-node --force
    cargo install --path rust-system/blockchain-cli --force
    
    print_success "Binaries installed"
}

# Create distribution package
create_package() {
    print_step "Creating distribution package..."
    
    local package_dir="hybrid-blockchain-$(date +%Y%m%d)"
    mkdir -p "dist/${package_dir}"
    
    # Copy binaries
    cp target/release/blockchain-node "dist/${package_dir}/"
    cp target/release/blockchain-cli "dist/${package_dir}/"
    
    # Copy libraries
    cp cpp-core/build/libblockchain_core.a "dist/${package_dir}/"
    cp cpp-core/build/libblockchain_ffi.a "dist/${package_dir}/"
    
    # Copy documentation
    cp -r docs "dist/${package_dir}/"
    cp README.md "dist/${package_dir}/"
    
    # Create archive
    cd dist
    tar -czf "${package_dir}.tar.gz" "${package_dir}"
    cd ..
    
    print_success "Package created: dist/${package_dir}.tar.gz"
}

# Run benchmarks
run_benchmarks() {
    print_step "Running performance benchmarks..."
    
    # C++ benchmarks
    if [ -f "cpp-core/build/blockchain_benchmarks" ]; then
        echo "C++ Performance Results:"
        ./cpp-core/build/blockchain_benchmarks
    fi
    
    # Rust benchmarks  
    echo "Rust Performance Results:"
    cargo bench --all
    
    print_success "Benchmarks completed"
}

# Clean build artifacts
clean() {
    print_step "Cleaning build artifacts..."
    
    # Clean C++ build
    rm -rf cpp-core/build
    
    # Clean Rust build
    cargo clean
    
    # Clean distribution
    rm -rf dist
    
    print_success "Build artifacts cleaned"
}

# Main execution
case "${1:-all}" in
    "deps")
        check_requirements
        ;;
    "cpp")
        check_requirements
        build_cpp_core
        ;;
    "rust")
        check_requirements
        build_rust_system
        ;;
    "test")
        run_integration_tests
        ;;
    "docs")
        generate_docs
        ;;
    "install")
        install_binaries
        ;;
    "package")
        create_package
        ;;
    "bench")
        run_benchmarks
        ;;
    "clean")
        clean
        ;;
    "all")
        check_requirements
        build_cpp_core
        build_rust_system
        run_integration_tests
        print_success "🎉 Hybrid blockchain system built successfully!"
        echo ""
        echo "Available executables:"
        echo "  • target/release/blockchain-node  - Full blockchain node"
        echo "  • target/release/blockchain-cli   - Command-line interface"
        echo ""
        echo "Run './build.sh install' to install system-wide"
        echo "Run './build.sh bench' to run performance benchmarks"
        ;;
    *)
        echo "Usage: $0 [deps|cpp|rust|test|docs|install|package|bench|clean|all]"
        echo ""
        echo "Commands:"
        echo "  deps     - Check and install system dependencies"
        echo "  cpp      - Build C++ core engine only"
        echo "  rust     - Build Rust system components only"
        echo "  test     - Run integration tests"
        echo "  docs     - Generate documentation"
        echo "  install  - Install binaries system-wide"
        echo "  package  - Create distribution package"
        echo "  bench    - Run performance benchmarks"
        echo "  clean    - Clean all build artifacts"
        echo "  all      - Build everything (default)"
        echo ""
        echo "Environment variables:"
        echo "  BUILD_TYPE      - Debug or Release (default: Release)"
        echo "  PARALLEL_JOBS   - Number of parallel build jobs (default: nproc)"
        echo "  INSTALL_DEPS    - Auto-install dependencies (default: false)"
        exit 1
        ;;
esac