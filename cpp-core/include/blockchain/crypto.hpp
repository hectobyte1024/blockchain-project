#pragma once

#include <array>
#include <vector>
#include <string>
#include <memory>
#include <cstdint>
#include <optional>

namespace blockchain::crypto {

// Type aliases for cryptographic primitives
using Hash256 = std::array<uint8_t, 32>;
using Hash160 = std::array<uint8_t, 20>;
using PrivateKey = std::array<uint8_t, 32>;
using PublicKey = std::array<uint8_t, 33>; // Compressed
using Signature = std::array<uint8_t, 64>; // r + s components

/**
 * @brief SHA-256 hash function implementation
 */
class SHA256 {
public:
    static Hash256 hash(const std::vector<uint8_t>& data);
    static Hash256 hash(const uint8_t* data, size_t length);
    static Hash256 double_hash(const std::vector<uint8_t>& data); // SHA256(SHA256(x))
    
    // Streaming interface for large data
    class Hasher {
    public:
        Hasher();
        ~Hasher();
        
        void update(const uint8_t* data, size_t length);
        void update(const std::vector<uint8_t>& data);
        Hash256 finalize();
        
    private:
        class Impl;
        std::unique_ptr<Impl> impl_;
    };
};

/**
 * @brief RIPEMD-160 hash function
 */
class RIPEMD160 {
public:
    static Hash160 hash(const std::vector<uint8_t>& data);
    static Hash160 hash(const uint8_t* data, size_t length);
};

/**
 * @brief ECDSA cryptographic operations using secp256k1 curve
 */
class ECDSA {
public:
    // Key generation
    static PrivateKey generate_private_key();
    static std::optional<PublicKey> derive_public_key(const PrivateKey& private_key);
    
    // Digital signatures
    static std::optional<Signature> sign(const Hash256& message_hash, const PrivateKey& private_key);
    static bool verify(const Hash256& message_hash, const Signature& signature, const PublicKey& public_key);
    
    // Key validation
    static bool is_valid_private_key(const PrivateKey& key);
    static bool is_valid_public_key(const PublicKey& key);
    
    // Key recovery from signature (used in Ethereum-style transactions)
    static std::optional<PublicKey> recover_public_key(const Hash256& message_hash, const Signature& signature, int recovery_id);
};

/**
 * @brief Merkle tree implementation for transaction aggregation
 */
class MerkleTree {
public:
    explicit MerkleTree(const std::vector<Hash256>& leaf_hashes);
    
    Hash256 get_root() const;
    std::vector<Hash256> get_proof(size_t leaf_index) const;
    
    static bool verify_proof(const Hash256& leaf_hash, const std::vector<Hash256>& proof, 
                           const Hash256& root, size_t leaf_index, size_t tree_size);
    
private:
    std::vector<std::vector<Hash256>> levels_;
    void build_tree();
};

/**
 * @brief Base58 encoding/decoding (used for Bitcoin-style addresses)
 */
class Base58 {
public:
    static std::string encode(const std::vector<uint8_t>& data);
    static std::optional<std::vector<uint8_t>> decode(const std::string& encoded);
    
    // Base58Check (includes checksum)
    static std::string encode_check(const std::vector<uint8_t>& data);
    static std::optional<std::vector<uint8_t>> decode_check(const std::string& encoded);
};

/**
 * @brief Bech32 encoding/decoding (used for SegWit addresses)
 */
class Bech32 {
public:
    static std::string encode(const std::string& hrp, const std::vector<uint8_t>& data);
    static std::optional<std::pair<std::string, std::vector<uint8_t>>> decode(const std::string& encoded);
};

/**
 * @brief HMAC (Hash-based Message Authentication Code)
 */
class HMAC {
public:
    static Hash256 hmac_sha256(const std::vector<uint8_t>& key, const std::vector<uint8_t>& message);
    static Hash256 hmac_sha512(const std::vector<uint8_t>& key, const std::vector<uint8_t>& message);
};

/**
 * @brief PBKDF2 key derivation function
 */
class PBKDF2 {
public:
    static std::vector<uint8_t> derive_key(const std::string& password, 
                                         const std::vector<uint8_t>& salt,
                                         int iterations, 
                                         size_t key_length);
};

/**
 * @brief Cryptographically secure random number generation
 */
class SecureRandom {
public:
    static std::vector<uint8_t> generate_bytes(size_t count);
    static uint32_t generate_uint32();
    static uint64_t generate_uint64();
};

/**
 * @brief Hash256 hasher for std::unordered_map
 */
struct Hash256Hasher {
    std::size_t operator()(const Hash256& hash) const noexcept {
        // Use the first 8 bytes as hash value for good distribution
        std::size_t result = 0;
        for (size_t i = 0; i < 8 && i < hash.size(); ++i) {
            result = (result << 8) | hash[i];
        }
        return result;
    }
};

/**
 * @brief Utility functions for cryptographic operations
 */
namespace utils {
    std::string to_hex(const std::vector<uint8_t>& data);
    std::string to_hex(const Hash256& hash);
    std::string to_hex(const Hash160& hash);
    
    std::optional<std::vector<uint8_t>> from_hex(const std::string& hex);
    std::optional<Hash256> hash256_from_hex(const std::string& hex);
    std::optional<Hash160> hash160_from_hex(const std::string& hex);
    
    // Constant-time comparison to prevent timing attacks
    bool secure_compare(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b);
}

} // namespace blockchain::crypto