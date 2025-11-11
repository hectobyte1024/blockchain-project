cmake_minimum_required(VERSION 3.20)
project(HybridBlockchainCore VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Build options
option(BUILD_TESTS "Build tests" ON)
option(BUILD_BENCHMARKS "Build benchmarks" ON)
option(ENABLE_ASAN "Enable AddressSanitizer" OFF)
option(ENABLE_TSAN "Enable ThreadSanitizer" OFF)

# Compiler flags
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0 -Wall -Wextra -Wpedantic")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG -march=native")

if(ENABLE_ASAN)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fsanitize=address -fno-omit-frame-pointer")
    set(CMAKE_LINKER_FLAGS "${CMAKE_LINKER_FLAGS} -fsanitize=address")
endif()

if(ENABLE_TSAN)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fsanitize=thread")
    set(CMAKE_LINKER_FLAGS "${CMAKE_LINKER_FLAGS} -fsanitize=thread")
endif()

# Find required packages
find_package(PkgConfig REQUIRED)
find_package(Threads REQUIRED)
find_package(OpenSSL REQUIRED)

# Find Boost
# Boost libraries (for networking and serialization) - temporarily disabled for consensus module
# find_package(Boost 1.75.0 REQUIRED COMPONENTS 
#     system 
#     filesystem 
#     program_options 
#     thread 
#     serialization 
#     iostreams
#     chrono
#     date_time
#     atomic
#     regex
#     asio
# )

# Find libsecp256k1
pkg_check_modules(SECP256K1 REQUIRED libsecp256k1)

# Find LevelDB
find_path(LEVELDB_INCLUDE_DIR leveldb/db.h)
find_library(LEVELDB_LIBRARY leveldb)
if(NOT LEVELDB_INCLUDE_DIR OR NOT LEVELDB_LIBRARY)
    message(WARNING "LevelDB not found, storage features will be limited")
endif()

# Include directories
include_directories(${CMAKE_SOURCE_DIR}/include)
include_directories(${CMAKE_SOURCE_DIR}/src)
include_directories(${Boost_INCLUDE_DIRS})
include_directories(${LEVELDB_INCLUDE_DIR})

# Core library source files (performance-critical components)
set(CORE_SOURCES
    "src/crypto/crypto.cpp"
    "src/consensus/consensus.cpp"
    "src/consensus/simple_consensus.cpp"
    "src/blockchain/block.cpp"
    "src/blockchain/transaction.cpp"
    "src/storage/storage_utils.cpp"
    "src/vm/vm.cpp"
    "src/vm/vm_test.cpp"
)

# FFI interface source files
file(GLOB_RECURSE FFI_SOURCES 
    "src/ffi/*.cpp"
)

# Create the main core library (static for embedding in Rust)
add_library(blockchain_core STATIC ${CORE_SOURCES})

target_link_libraries(blockchain_core 
    # ${Boost_LIBRARIES}  # Temporarily disabled
    OpenSSL::SSL 
    OpenSSL::Crypto
    Threads::Threads
)

# Add LevelDB if found
if(LEVELDB_LIBRARY)
    target_link_libraries(blockchain_core ${LEVELDB_LIBRARY})
    target_compile_definitions(blockchain_core PRIVATE HAVE_LEVELDB=1)
else()
    target_compile_definitions(blockchain_core PRIVATE HAVE_LEVELDB=0)
endif()

# Create the FFI interface library (also static)
add_library(blockchain_ffi STATIC ${FFI_SOURCES})
target_link_libraries(blockchain_ffi blockchain_core)

# Note: Executables are now built in Rust - C++ provides core engine only

# Tests - Temporarily disabled until Google Test is properly installed
if(BUILD_TESTS AND FALSE)  # Force disable for now
    enable_testing()
    find_package(GTest REQUIRED)
    
    file(GLOB_RECURSE TEST_SOURCES "tests/*.cpp")
    add_executable(blockchain_tests ${TEST_SOURCES})
    target_link_libraries(blockchain_tests 
        blockchain_core 
        GTest::gtest 
        GTest::gtest_main
    )
    
    add_test(NAME BlockchainTests COMMAND blockchain_tests)
endif()

# Benchmarks
if(BUILD_BENCHMARKS)
    # find_package(benchmark REQUIRED)  # Temporarily disabled
    
    file(GLOB_RECURSE BENCH_SOURCES "benchmarks/*.cpp")
    if(BENCH_SOURCES)
        add_executable(blockchain_benchmarks ${BENCH_SOURCES})
        target_link_libraries(blockchain_benchmarks 
            blockchain_core 
            benchmark::benchmark
        )
    endif()
endif()

# Installation for FFI integration
install(DIRECTORY include/ DESTINATION include)
install(TARGETS blockchain_core blockchain_ffi
    ARCHIVE DESTINATION lib
    LIBRARY DESTINATION lib
)

# Export compile commands for IDE integration
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)