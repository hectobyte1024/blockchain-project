#include <gtest/gtest.h>
#include "blockchain/crypto.hpp"
#include <vector>
#include <string>

using namespace blockchain::crypto;

class CryptoTest : public ::testing::Test {
protected:
    void SetUp() override {
        // Test setup
    }
    
    void TearDown() override {
        // Test cleanup
    }
};

TEST_F(CryptoTest, SHA256Hashing) {
    std::string test_string = "Hello, Blockchain!";
    std::vector<uint8_t> data(test_string.begin(), test_string.end());
    
    auto hash = SHA256::hash(data);
    
    // Convert to hex for verification
    std::string hex = utils::to_hex(hash);
    
    // Hash should be 64 characters (32 bytes * 2)
    EXPECT_EQ(hex.length(), 64);
    
    // Hash should be deterministic
    auto hash2 = SHA256::hash(data);
    EXPECT_EQ(hash, hash2);
}

TEST_F(CryptoTest, DoubleSHA256) {
    std::string test_string = "Bitcoin";
    std::vector<uint8_t> data(test_string.begin(), test_string.end());
    
    auto single_hash = SHA256::hash(data);
    auto double_hash = SHA256::double_hash(data);
    
    // Double hash should be different from single hash
    EXPECT_NE(single_hash, double_hash);
    
    // Double hash should be SHA256(SHA256(data))
    auto manual_double = SHA256::hash(std::vector<uint8_t>(single_hash.begin(), single_hash.end()));
    EXPECT_EQ(double_hash, manual_double);
}

TEST_F(CryptoTest, RIPEMD160Hashing) {
    std::string test_string = "Test RIPEMD160";
    std::vector<uint8_t> data(test_string.begin(), test_string.end());
    
    auto hash = RIPEMD160::hash(data);
    
    // Convert to hex
    std::string hex = utils::to_hex(hash);
    
    // Hash should be 40 characters (20 bytes * 2)
    EXPECT_EQ(hex.length(), 40);
}

TEST_F(CryptoTest, ECDSAKeyGeneration) {
    // Generate private key
    auto private_key = ECDSA::generate_private_key();
    
    // Validate private key
    EXPECT_TRUE(ECDSA::is_valid_private_key(private_key));
    
    // Derive public key
    auto public_key = ECDSA::derive_public_key(private_key);
    ASSERT_TRUE(public_key.has_value());
    
    // Validate public key
    EXPECT_TRUE(ECDSA::is_valid_public_key(public_key.value()));
    
    // Public key should be 33 bytes (compressed)
    EXPECT_EQ(public_key->size(), 33);
    
    // First byte should indicate compressed key (0x02 or 0x03)
    EXPECT_TRUE((*public_key)[0] == 0x02 || (*public_key)[0] == 0x03);
}

TEST_F(CryptoTest, ECDSASignAndVerify) {
    // Generate keys
    auto private_key = ECDSA::generate_private_key();
    auto public_key = ECDSA::derive_public_key(private_key);
    ASSERT_TRUE(public_key.has_value());
    
    // Create message to sign
    std::string message = "Sign this message";
    std::vector<uint8_t> message_data(message.begin(), message.end());
    auto message_hash = SHA256::hash(message_data);
    
    // Sign message
    auto signature = ECDSA::sign(message_hash, private_key);
    ASSERT_TRUE(signature.has_value());
    
    // Verify signature
    bool is_valid = ECDSA::verify(message_hash, signature.value(), public_key.value());
    EXPECT_TRUE(is_valid);
    
    // Verify with wrong message should fail
    std::string wrong_message = "Wrong message";
    std::vector<uint8_t> wrong_data(wrong_message.begin(), wrong_message.end());
    auto wrong_hash = SHA256::hash(wrong_data);
    
    bool wrong_verification = ECDSA::verify(wrong_hash, signature.value(), public_key.value());
    EXPECT_FALSE(wrong_verification);
}

TEST_F(CryptoTest, MerkleTreeConstruction) {
    // Create test hashes
    std::vector<Hash256> leaf_hashes;
    for (int i = 0; i < 4; ++i) {
        std::string data = "Leaf " + std::to_string(i);
        std::vector<uint8_t> leaf_data(data.begin(), data.end());
        leaf_hashes.push_back(SHA256::hash(leaf_data));
    }
    
    // Build merkle tree
    MerkleTree tree(leaf_hashes);
    auto root = tree.get_root();
    
    // Root should not be zero
    Hash256 zero_hash = {};
    EXPECT_NE(root, zero_hash);
    
    // Generate proof for first leaf
    auto proof = tree.get_proof(0);
    EXPECT_FALSE(proof.empty());
    
    // Verify proof
    bool proof_valid = MerkleTree::verify_proof(leaf_hashes[0], proof, root, 0, leaf_hashes.size());
    EXPECT_TRUE(proof_valid);
    
    // Invalid proof should fail
    bool invalid_proof = MerkleTree::verify_proof(leaf_hashes[1], proof, root, 0, leaf_hashes.size());
    EXPECT_FALSE(invalid_proof);
}

TEST_F(CryptoTest, Base58Encoding) {
    // Test known Base58 encoding
    std::vector<uint8_t> test_data = {0x00, 0x01, 0x02, 0x03, 0x04};
    
    std::string encoded = Base58::encode(test_data);
    EXPECT_FALSE(encoded.empty());
    
    auto decoded = Base58::decode(encoded);
    ASSERT_TRUE(decoded.has_value());
    EXPECT_EQ(test_data, decoded.value());
    
    // Test with leading zeros
    std::vector<uint8_t> zero_data = {0x00, 0x00, 0x01, 0x02};
    std::string zero_encoded = Base58::encode(zero_data);
    
    // Should start with '1's for leading zeros
    EXPECT_TRUE(zero_encoded.starts_with(\"11\"));
    
    auto zero_decoded = Base58::decode(zero_encoded);
    ASSERT_TRUE(zero_decoded.has_value());
    EXPECT_EQ(zero_data, zero_decoded.value());
}

TEST_F(CryptoTest, Base58Check) {
    std::vector<uint8_t> test_data = {0x76, 0xa9, 0x14}; // P2PKH prefix
    
    std::string encoded = Base58::encode_check(test_data);
    EXPECT_FALSE(encoded.empty());
    
    auto decoded = Base58::decode_check(encoded);
    ASSERT_TRUE(decoded.has_value());
    EXPECT_EQ(test_data, decoded.value());
    
    // Corrupted checksum should fail
    std::string corrupted = encoded;
    corrupted.back() = '1'; // Change last character
    
    auto corrupted_decoded = Base58::decode_check(corrupted);
    EXPECT_FALSE(corrupted_decoded.has_value());
}

TEST_F(CryptoTest, HMAC) {
    std::vector<uint8_t> key = {0x01, 0x02, 0x03, 0x04};
    std::vector<uint8_t> message = {0x05, 0x06, 0x07, 0x08};
    
    auto hmac_result = HMAC::hmac_sha256(key, message);
    
    // HMAC should be deterministic
    auto hmac_result2 = HMAC::hmac_sha256(key, message);
    EXPECT_EQ(hmac_result, hmac_result2);
    
    // Different key should produce different result
    std::vector<uint8_t> different_key = {0x01, 0x02, 0x03, 0x05};
    auto different_hmac = HMAC::hmac_sha256(different_key, message);
    EXPECT_NE(hmac_result, different_hmac);
}

TEST_F(CryptoTest, PBKDF2) {
    std::string password = "test_password";
    std::vector<uint8_t> salt = {0x01, 0x02, 0x03, 0x04};
    int iterations = 1000;
    size_t key_length = 32;
    
    auto derived_key = PBKDF2::derive_key(password, salt, iterations, key_length);
    
    EXPECT_EQ(derived_key.size(), key_length);
    
    // Same parameters should produce same key
    auto derived_key2 = PBKDF2::derive_key(password, salt, iterations, key_length);
    EXPECT_EQ(derived_key, derived_key2);
    
    // Different salt should produce different key
    std::vector<uint8_t> different_salt = {0x01, 0x02, 0x03, 0x05};
    auto different_key = PBKDF2::derive_key(password, different_salt, iterations, key_length);
    EXPECT_NE(derived_key, different_key);
}

TEST_F(CryptoTest, SecureRandom) {
    // Generate random bytes
    auto random1 = SecureRandom::generate_bytes(32);
    auto random2 = SecureRandom::generate_bytes(32);
    
    EXPECT_EQ(random1.size(), 32);
    EXPECT_EQ(random2.size(), 32);
    
    // Should be different (extremely high probability)
    EXPECT_NE(random1, random2);
    
    // Generate random integers
    auto rand_int1 = SecureRandom::generate_uint32();
    auto rand_int2 = SecureRandom::generate_uint32();
    
    // Should be different (high probability)
    EXPECT_NE(rand_int1, rand_int2);
}

TEST_F(CryptoTest, HexUtils) {
    std::vector<uint8_t> test_data = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
    
    std::string hex = utils::to_hex(test_data);
    EXPECT_EQ(hex, "0123456789abcdef");
    
    auto decoded = utils::from_hex(hex);
    ASSERT_TRUE(decoded.has_value());
    EXPECT_EQ(test_data, decoded.value());
    
    // Test case insensitivity
    auto decoded_upper = utils::from_hex("0123456789ABCDEF");
    ASSERT_TRUE(decoded_upper.has_value());
    EXPECT_EQ(test_data, decoded_upper.value());
    
    // Invalid hex should return nullopt
    auto invalid = utils::from_hex("gg");
    EXPECT_FALSE(invalid.has_value());
    
    // Odd length should return nullopt
    auto odd_length = utils::from_hex("123");
    EXPECT_FALSE(odd_length.has_value());
}

TEST_F(CryptoTest, SecureCompare) {
    std::vector<uint8_t> data1 = {0x01, 0x02, 0x03, 0x04};
    std::vector<uint8_t> data2 = {0x01, 0x02, 0x03, 0x04};
    std::vector<uint8_t> data3 = {0x01, 0x02, 0x03, 0x05};
    std::vector<uint8_t> data4 = {0x01, 0x02, 0x03}; // Different length
    
    EXPECT_TRUE(utils::secure_compare(data1, data2));
    EXPECT_FALSE(utils::secure_compare(data1, data3));
    EXPECT_FALSE(utils::secure_compare(data1, data4));
}

// Performance test for cryptographic operations
TEST_F(CryptoTest, PerformanceTest) {
    const int iterations = 1000;
    
    auto start = std::chrono::high_resolution_clock::now();
    
    // Test SHA-256 performance
    std::vector<uint8_t> data(1024, 0x42); // 1KB of data
    for (int i = 0; i < iterations; ++i) {
        SHA256::hash(data);
    }
    
    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
    
    // Should complete in reasonable time (less than 1 second for 1000 iterations)
    EXPECT_LT(duration.count(), 1000000);
    
    std::cout << "SHA-256 performance: " << iterations << " iterations in " 
              << duration.count() << " microseconds" << std::endl;
}