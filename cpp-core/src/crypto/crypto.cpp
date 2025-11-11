#include "blockchain/crypto.hpp"
#include "ffi/blockchain_ffi.h"
#include <openssl/sha.h>
#include <openssl/ripemd.h>
#include <openssl/rand.h>
#include <openssl/evp.h>
#include <openssl/hmac.h>
#include <secp256k1.h>
#include <secp256k1_recovery.h>
#include <algorithm>
#include <cstring>
#include <memory>
#include <mutex>

namespace blockchain::crypto {

// Global secp256k1 context (thread-safe)
static secp256k1_context* g_secp256k1_context = nullptr;
static std::once_flag g_secp256k1_init_flag;

static void init_secp256k1_context() {
    std::call_once(g_secp256k1_init_flag, []() {
        g_secp256k1_context = secp256k1_context_create(
            SECP256K1_CONTEXT_SIGN | SECP256K1_CONTEXT_VERIFY
        );
        
        // Add randomness to the context
        unsigned char seed[32];
        if (RAND_bytes(seed, 32) != 1) {
            throw std::runtime_error("Failed to generate random seed for secp256k1");
        }
        secp256k1_context_randomize(g_secp256k1_context, seed);
    });
}

// SHA-256 Implementation
Hash256 SHA256::hash(const std::vector<uint8_t>& data) {
    return hash(data.data(), data.size());
}

Hash256 SHA256::hash(const uint8_t* data, size_t length) {
    Hash256 result;
    SHA256_CTX ctx;
    SHA256_Init(&ctx);
    SHA256_Update(&ctx, data, length);
    SHA256_Final(result.data(), &ctx);
    return result;
}

Hash256 SHA256::double_hash(const std::vector<uint8_t>& data) {
    Hash256 first_hash = hash(data);
    return hash(first_hash.data(), first_hash.size());
}

// SHA-256 Hasher implementation
class SHA256::Hasher::Impl {
public:
    SHA256_CTX ctx;
    
    Impl() {
        SHA256_Init(&ctx);
    }
};

SHA256::Hasher::Hasher() : impl_(std::make_unique<Impl>()) {}

SHA256::Hasher::~Hasher() = default;

void SHA256::Hasher::update(const uint8_t* data, size_t length) {
    SHA256_Update(&impl_->ctx, data, length);
}

void SHA256::Hasher::update(const std::vector<uint8_t>& data) {
    update(data.data(), data.size());
}

Hash256 SHA256::Hasher::finalize() {
    Hash256 result;
    SHA256_Final(result.data(), &impl_->ctx);
    return result;
}

// RIPEMD-160 Implementation
Hash160 RIPEMD160::hash(const std::vector<uint8_t>& data) {
    return hash(data.data(), data.size());
}

Hash160 RIPEMD160::hash(const uint8_t* data, size_t length) {
    Hash160 result;
    RIPEMD160_CTX ctx;
    RIPEMD160_Init(&ctx);
    RIPEMD160_Update(&ctx, data, length);
    RIPEMD160_Final(result.data(), &ctx);
    return result;
}

// ECDSA Implementation
PrivateKey ECDSA::generate_private_key() {
    init_secp256k1_context();
    
    PrivateKey private_key;
    do {
        if (RAND_bytes(private_key.data(), 32) != 1) {
            throw std::runtime_error("Failed to generate random bytes for private key");
        }
    } while (!secp256k1_ec_seckey_verify(g_secp256k1_context, private_key.data()));
    
    return private_key;
}

std::optional<PublicKey> ECDSA::derive_public_key(const PrivateKey& private_key) {
    init_secp256k1_context();
    
    if (!is_valid_private_key(private_key)) {
        return std::nullopt;
    }
    
    secp256k1_pubkey pubkey;
    if (!secp256k1_ec_pubkey_create(g_secp256k1_context, &pubkey, private_key.data())) {
        return std::nullopt;
    }
    
    PublicKey compressed_pubkey;
    size_t output_len = 33;
    secp256k1_ec_pubkey_serialize(
        g_secp256k1_context,
        compressed_pubkey.data(),
        &output_len,
        &pubkey,
        SECP256K1_EC_COMPRESSED
    );
    
    return compressed_pubkey;
}

std::optional<Signature> ECDSA::sign(const Hash256& message_hash, const PrivateKey& private_key) {
    init_secp256k1_context();
    
    if (!is_valid_private_key(private_key)) {
        return std::nullopt;
    }
    
    secp256k1_ecdsa_signature sig;
    if (!secp256k1_ecdsa_sign(g_secp256k1_context, &sig, message_hash.data(), private_key.data(), nullptr, nullptr)) {
        return std::nullopt;
    }
    
    Signature compact_sig;
    secp256k1_ecdsa_signature_serialize_compact(g_secp256k1_context, compact_sig.data(), &sig);
    
    return compact_sig;
}

bool ECDSA::verify(const Hash256& message_hash, const Signature& signature, const PublicKey& public_key) {
    init_secp256k1_context();
    
    if (!is_valid_public_key(public_key)) {
        return false;
    }
    
    secp256k1_pubkey pubkey;
    if (!secp256k1_ec_pubkey_parse(g_secp256k1_context, &pubkey, public_key.data(), 33)) {
        return false;
    }
    
    secp256k1_ecdsa_signature sig;
    if (!secp256k1_ecdsa_signature_parse_compact(g_secp256k1_context, &sig, signature.data())) {
        return false;
    }
    
    return secp256k1_ecdsa_verify(g_secp256k1_context, &sig, message_hash.data(), &pubkey) == 1;
}

bool ECDSA::is_valid_private_key(const PrivateKey& key) {
    init_secp256k1_context();
    return secp256k1_ec_seckey_verify(g_secp256k1_context, key.data()) == 1;
}

bool ECDSA::is_valid_public_key(const PublicKey& key) {
    init_secp256k1_context();
    secp256k1_pubkey pubkey;
    return secp256k1_ec_pubkey_parse(g_secp256k1_context, &pubkey, key.data(), 33) == 1;
}

std::optional<PublicKey> ECDSA::recover_public_key(const Hash256& message_hash, const Signature& signature, int recovery_id) {
    init_secp256k1_context();
    
    if (recovery_id < 0 || recovery_id > 3) {
        return std::nullopt;
    }
    
    secp256k1_ecdsa_recoverable_signature recoverable_sig;
    if (!secp256k1_ecdsa_recoverable_signature_parse_compact(
            g_secp256k1_context, &recoverable_sig, signature.data(), recovery_id)) {
        return std::nullopt;
    }
    
    secp256k1_pubkey pubkey;
    if (!secp256k1_ecdsa_recover(g_secp256k1_context, &pubkey, &recoverable_sig, message_hash.data())) {
        return std::nullopt;
    }
    
    PublicKey compressed_pubkey;
    size_t output_len = 33;
    secp256k1_ec_pubkey_serialize(
        g_secp256k1_context,
        compressed_pubkey.data(),
        &output_len,
        &pubkey,
        SECP256K1_EC_COMPRESSED
    );
    
    return compressed_pubkey;
}

// Merkle Tree Implementation
MerkleTree::MerkleTree(const std::vector<Hash256>& leaf_hashes) {
    if (leaf_hashes.empty()) {
        return;
    }
    
    levels_.push_back(leaf_hashes);
    build_tree();
}

Hash256 MerkleTree::get_root() const {
    if (levels_.empty() || levels_.back().empty()) {
        return Hash256{}; // Zero hash
    }
    return levels_.back()[0];
}

void MerkleTree::build_tree() {
    while (levels_.back().size() > 1) {
        const auto& current_level = levels_.back();
        std::vector<Hash256> next_level;
        
        for (size_t i = 0; i < current_level.size(); i += 2) {
            Hash256 combined_hash;
            
            if (i + 1 < current_level.size()) {
                // Combine two hashes
                std::vector<uint8_t> combined;
                combined.reserve(64);
                combined.insert(combined.end(), current_level[i].begin(), current_level[i].end());
                combined.insert(combined.end(), current_level[i + 1].begin(), current_level[i + 1].end());
                combined_hash = SHA256::hash(combined);
            } else {
                // Odd number of elements, duplicate the last one
                std::vector<uint8_t> combined;
                combined.reserve(64);
                combined.insert(combined.end(), current_level[i].begin(), current_level[i].end());
                combined.insert(combined.end(), current_level[i].begin(), current_level[i].end());
                combined_hash = SHA256::hash(combined);
            }
            
            next_level.push_back(combined_hash);
        }
        
        levels_.push_back(next_level);
    }
}

std::vector<Hash256> MerkleTree::get_proof(size_t leaf_index) const {
    std::vector<Hash256> proof;
    
    if (levels_.empty() || leaf_index >= levels_[0].size()) {
        return proof;
    }
    
    size_t current_index = leaf_index;
    
    for (size_t level = 0; level < levels_.size() - 1; ++level) {
        const auto& current_level = levels_[level];
        
        size_t sibling_index;
        if (current_index % 2 == 0) {
            // Current node is left child, sibling is right
            sibling_index = current_index + 1;
        } else {
            // Current node is right child, sibling is left
            sibling_index = current_index - 1;
        }
        
        if (sibling_index < current_level.size()) {
            proof.push_back(current_level[sibling_index]);
        } else {
            // Odd number of nodes, duplicate the current node
            proof.push_back(current_level[current_index]);
        }
        
        current_index /= 2;
    }
    
    return proof;
}

bool MerkleTree::verify_proof(const Hash256& leaf_hash, const std::vector<Hash256>& proof, 
                             const Hash256& root, size_t leaf_index, size_t tree_size) {
    Hash256 current_hash = leaf_hash;
    size_t current_index = leaf_index;
    
    for (const auto& proof_hash : proof) {
        std::vector<uint8_t> combined;
        combined.reserve(64);
        
        if (current_index % 2 == 0) {
            // Current node is left child
            combined.insert(combined.end(), current_hash.begin(), current_hash.end());
            combined.insert(combined.end(), proof_hash.begin(), proof_hash.end());
        } else {
            // Current node is right child
            combined.insert(combined.end(), proof_hash.begin(), proof_hash.end());
            combined.insert(combined.end(), current_hash.begin(), current_hash.end());
        }
        
        current_hash = SHA256::hash(combined);
        current_index /= 2;
    }
    
    return current_hash == root;
}

// Base58 Implementation
static const char* BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

std::string Base58::encode(const std::vector<uint8_t>& data) {
    if (data.empty()) {
        return "";
    }
    
    // Count leading zeros
    size_t leading_zeros = 0;
    while (leading_zeros < data.size() && data[leading_zeros] == 0) {
        leading_zeros++;
    }
    
    // Convert to base58
    std::vector<uint8_t> temp(data.begin() + leading_zeros, data.end());
    std::string result;
    
    while (!temp.empty()) {
        uint32_t carry = 0;
        for (size_t i = 0; i < temp.size(); ++i) {
            carry = carry * 256 + temp[i];
            temp[i] = carry / 58;
            carry %= 58;
        }
        
        result = BASE58_ALPHABET[carry] + result;
        
        // Remove leading zeros from temp
        while (!temp.empty() && temp[0] == 0) {
            temp.erase(temp.begin());
        }
    }
    
    // Add leading '1's for leading zeros
    result = std::string(leading_zeros, '1') + result;
    
    return result;
}

std::optional<std::vector<uint8_t>> Base58::decode(const std::string& encoded) {
    if (encoded.empty()) {
        return std::vector<uint8_t>();
    }
    
    // Count leading '1's
    size_t leading_ones = 0;
    while (leading_ones < encoded.size() && encoded[leading_ones] == '1') {
        leading_ones++;
    }
    
    // Convert from base58
    std::vector<uint32_t> temp;
    for (size_t i = leading_ones; i < encoded.size(); ++i) {
        const char* pos = strchr(BASE58_ALPHABET, encoded[i]);
        if (!pos) {
            return std::nullopt; // Invalid character
        }
        
        uint32_t digit = pos - BASE58_ALPHABET;
        uint32_t carry = digit;
        
        for (size_t j = 0; j < temp.size(); ++j) {
            carry += temp[j] * 58;
            temp[j] = carry & 0xFF;
            carry >>= 8;
        }
        
        while (carry > 0) {
            temp.push_back(carry & 0xFF);
            carry >>= 8;
        }
    }
    
    // Convert to bytes and reverse
    std::vector<uint8_t> result(leading_ones, 0);
    for (auto it = temp.rbegin(); it != temp.rend(); ++it) {
        result.push_back(static_cast<uint8_t>(*it));
    }
    
    return result;
}

std::string Base58::encode_check(const std::vector<uint8_t>& data) {
    // Calculate checksum (first 4 bytes of double SHA-256)
    Hash256 hash = SHA256::double_hash(data);
    
    std::vector<uint8_t> data_with_checksum = data;
    data_with_checksum.insert(data_with_checksum.end(), hash.begin(), hash.begin() + 4);
    
    return encode(data_with_checksum);
}

std::optional<std::vector<uint8_t>> Base58::decode_check(const std::string& encoded) {
    auto decoded = decode(encoded);
    if (!decoded || decoded->size() < 4) {
        return std::nullopt;
    }
    
    // Extract data and checksum
    std::vector<uint8_t> data(decoded->begin(), decoded->end() - 4);
    std::vector<uint8_t> checksum(decoded->end() - 4, decoded->end());
    
    // Verify checksum
    Hash256 hash = SHA256::double_hash(data);
    if (!std::equal(checksum.begin(), checksum.end(), hash.begin())) {
        return std::nullopt; // Invalid checksum
    }
    
    return data;
}

// HMAC Implementation
Hash256 HMAC::hmac_sha256(const std::vector<uint8_t>& key, const std::vector<uint8_t>& message) {
    Hash256 result;
    
    HMAC_CTX* ctx = HMAC_CTX_new();
    HMAC_Init_ex(ctx, key.data(), key.size(), EVP_sha256(), nullptr);
    HMAC_Update(ctx, message.data(), message.size());
    
    unsigned int result_len;
    HMAC_Final(ctx, result.data(), &result_len);
    HMAC_CTX_free(ctx);
    
    return result;
}

Hash256 HMAC::hmac_sha512(const std::vector<uint8_t>& key, const std::vector<uint8_t>& message) {
    unsigned char full_result[64];
    
    HMAC_CTX* ctx = HMAC_CTX_new();
    HMAC_Init_ex(ctx, key.data(), key.size(), EVP_sha512(), nullptr);
    HMAC_Update(ctx, message.data(), message.size());
    
    unsigned int result_len;
    HMAC_Final(ctx, full_result, &result_len);
    HMAC_CTX_free(ctx);
    
    // Return only first 32 bytes as Hash256
    Hash256 result;
    std::copy(full_result, full_result + 32, result.begin());
    return result;
}

// PBKDF2 Implementation
std::vector<uint8_t> PBKDF2::derive_key(const std::string& password, 
                                       const std::vector<uint8_t>& salt,
                                       int iterations, 
                                       size_t key_length) {
    std::vector<uint8_t> derived_key(key_length);
    
    PKCS5_PBKDF2_HMAC(
        password.c_str(), password.length(),
        salt.data(), salt.size(),
        iterations,
        EVP_sha256(),
        key_length,
        derived_key.data()
    );
    
    return derived_key;
}

// SecureRandom Implementation
std::vector<uint8_t> SecureRandom::generate_bytes(size_t count) {
    std::vector<uint8_t> bytes(count);
    if (RAND_bytes(bytes.data(), count) != 1) {
        throw std::runtime_error("Failed to generate secure random bytes");
    }
    return bytes;
}

uint32_t SecureRandom::generate_uint32() {
    uint32_t value;
    if (RAND_bytes(reinterpret_cast<uint8_t*>(&value), sizeof(value)) != 1) {
        throw std::runtime_error("Failed to generate secure random uint32");
    }
    return value;
}

uint64_t SecureRandom::generate_uint64() {
    uint64_t value;
    if (RAND_bytes(reinterpret_cast<uint8_t*>(&value), sizeof(value)) != 1) {
        throw std::runtime_error("Failed to generate secure random uint64");
    }
    return value;
}

// Utility functions
namespace utils {

std::string to_hex(const std::vector<uint8_t>& data) {
    std::string hex;
    hex.reserve(data.size() * 2);
    
    static const char hex_chars[] = "0123456789abcdef";
    for (uint8_t byte : data) {
        hex.push_back(hex_chars[byte >> 4]);
        hex.push_back(hex_chars[byte & 0x0F]);
    }
    
    return hex;
}

std::string to_hex(const Hash256& hash) {
    return to_hex(std::vector<uint8_t>(hash.begin(), hash.end()));
}

std::string to_hex(const Hash160& hash) {
    return to_hex(std::vector<uint8_t>(hash.begin(), hash.end()));
}

std::optional<std::vector<uint8_t>> from_hex(const std::string& hex) {
    if (hex.length() % 2 != 0) {
        return std::nullopt;
    }
    
    std::vector<uint8_t> bytes;
    bytes.reserve(hex.length() / 2);
    
    for (size_t i = 0; i < hex.length(); i += 2) {
        char high = hex[i];
        char low = hex[i + 1];
        
        auto hex_to_nibble = [](char c) -> std::optional<uint8_t> {
            if (c >= '0' && c <= '9') return c - '0';
            if (c >= 'a' && c <= 'f') return c - 'a' + 10;
            if (c >= 'A' && c <= 'F') return c - 'A' + 10;
            return std::nullopt;
        };
        
        auto high_nibble = hex_to_nibble(high);
        auto low_nibble = hex_to_nibble(low);
        
        if (!high_nibble || !low_nibble) {
            return std::nullopt;
        }
        
        bytes.push_back((*high_nibble << 4) | *low_nibble);
    }
    
    return bytes;
}

std::optional<Hash256> hash256_from_hex(const std::string& hex) {
    auto bytes = from_hex(hex);
    if (!bytes || bytes->size() != 32) {
        return std::nullopt;
    }
    
    Hash256 hash;
    std::copy(bytes->begin(), bytes->end(), hash.begin());
    return hash;
}

std::optional<Hash160> hash160_from_hex(const std::string& hex) {
    auto bytes = from_hex(hex);
    if (!bytes || bytes->size() != 20) {
        return std::nullopt;
    }
    
    Hash160 hash;
    std::copy(bytes->begin(), bytes->end(), hash.begin());
    return hash;
}

bool secure_compare(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b) {
    if (a.size() != b.size()) {
        return false;
    }
    
    volatile uint8_t result = 0;
    for (size_t i = 0; i < a.size(); ++i) {
        result |= a[i] ^ b[i];
    }
    
    return result == 0;
}

} // namespace utils

} // namespace blockchain::crypto