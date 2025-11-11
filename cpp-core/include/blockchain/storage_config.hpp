#pragma once

// Conditional compilation for storage backends
#ifndef HAVE_LEVELDB
#define HAVE_LEVELDB 0
#endif

#if HAVE_LEVELDB
#include <leveldb/db.h>
#include <leveldb/options.h>
#include <leveldb/write_batch.h>
#include <leveldb/cache.h>
#include <leveldb/filter_policy.h>
#define LEVELDB_AVAILABLE 1
#else
// Stub definitions for when LevelDB is not available
namespace leveldb {
    class DB;
    class Options;
    class WriteBatch;
    class Cache;
    class FilterPolicy;
    class Status;
    class ReadOptions;
    class WriteOptions;
}
#define LEVELDB_AVAILABLE 0
#endif

namespace blockchain {
namespace storage {

/// Check if LevelDB is available at compile time
constexpr bool is_leveldb_available() {
    return LEVELDB_AVAILABLE == 1;
}

} // namespace storage
} // namespace blockchain