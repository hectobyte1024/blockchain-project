use blockchain_network::{
    NetworkManager, NetworkConfig,
    protocol::{Message, MessageType, NetworkAddress, services},
    peer::{Peer, PeerInfo},
    discovery::AddressManager,
    tx_broadcast::{TransactionBroadcaster, TransactionPriority},
    NetworkError, NetworkStats,
};
use blockchain_core::{
    transaction::{Transaction, TransactionInput, TransactionOutput},
    block::Block,
    Hash256,
};
use std::{
    net::SocketAddr,
    time::Duration,
};
use tokio::time::timeout;
use uuid::Uuid;

#[tokio::test]
async fn test_network_config() {
    let config = NetworkConfig::default();
    
    assert_eq!(config.max_peers, 125);
    assert_eq!(config.listening_port, 8333);
    assert!(!config.dns_seeds.is_empty());
    assert_eq!(config.our_services, services::NODE_NETWORK);
    
    println!("✓ NetworkConfig validation passed");
}

#[tokio::test]
async fn test_network_manager_creation() {
    let config = NetworkConfig {
        listen_addr: "127.0.0.1:18333".parse().unwrap(),
        max_peers: 50,
        connection_timeout: Duration::from_secs(5),
        heartbeat_interval: Duration::from_secs(15),
        max_message_size: 16 * 1024 * 1024,
        network_magic: 0x0709110B, // Testnet
        dns_seeds: vec!["localhost".to_string()],
        seed_peers: vec!["127.0.0.1:8333".parse().unwrap()],
        our_services: services::NODE_NETWORK,
        listening_port: 18333,
    };
    
    let manager = NetworkManager::new(config);
    assert!(manager.is_ok());
    
    let mut manager = manager.unwrap();
    let event_receiver = manager.take_event_receiver();
    assert!(event_receiver.is_some());
    
    println!("✓ NetworkManager creation passed");
}

#[tokio::test]
async fn test_protocol_messages() {
    // Test Version message
    let local_addr = NetworkAddress::from_ipv4([127, 0, 0, 1], 8333, services::NODE_NETWORK);
    let remote_addr = NetworkAddress::from_ipv4([192, 168, 1, 100], 8333, services::NODE_NETWORK);
    
    let version_msg = Message::version(
        70015, // protocol version
        services::NODE_NETWORK,
        "EduBlockchain/1.0".to_string(),
        12345, // start height
        remote_addr.clone(),
        local_addr.clone(),
    );
    
    assert_eq!(version_msg.message_type, MessageType::Version);
    println!("✓ Version message created successfully");
    
    // Test serialization/deserialization
    let serialized = version_msg.serialize().unwrap();
    let deserialized = Message::deserialize(&serialized).unwrap();
    
    assert_eq!(version_msg.message_type, deserialized.message_type);
    println!("✓ Message serialization/deserialization passed");
    
    // Test other message types
    let ping_msg = Message::ping();
    assert_eq!(ping_msg.message_type, MessageType::Ping);
    
    let pong_msg = Message::pong(12345);
    assert_eq!(pong_msg.message_type, MessageType::Pong);
    
    let getaddr_msg = Message::getaddr();
    assert_eq!(getaddr_msg.message_type, MessageType::GetAddr);
    
    let addr_msg = Message::addr(vec![local_addr, remote_addr]);
    assert_eq!(addr_msg.message_type, MessageType::Addr);
    
    println!("✓ All protocol messages validated");
}

#[tokio::test]
async fn test_network_address() {
    // Test IPv4 address
    let ipv4_addr = NetworkAddress::from_ipv4([192, 168, 1, 1], 8333, services::NODE_NETWORK);
    
    assert!(ipv4_addr.is_ipv4());
    assert_eq!(ipv4_addr.get_ipv4(), Some([192, 168, 1, 1]));
    assert_eq!(ipv4_addr.port, 8333);
    assert_eq!(ipv4_addr.services, services::NODE_NETWORK);
    
    println!("✓ NetworkAddress IPv4 handling passed");
    
    // Test IPv6 address
    let ipv6_bytes = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let ipv6_addr = NetworkAddress::new(ipv6_bytes, 8333, services::NODE_NETWORK);
    
    assert!(!ipv6_addr.is_ipv4());
    assert_eq!(ipv6_addr.get_ipv4(), None);
    assert_eq!(ipv6_addr.ip, ipv6_bytes);
    
    println!("✓ NetworkAddress IPv6 handling passed");
}

#[tokio::test]
async fn test_peer_info() {
    let peer_id = Uuid::new_v4();
    let address: SocketAddr = "192.168.1.100:8333".parse().unwrap();
    let user_agent = "TestNode/1.0".to_string();
    
    let peer_info = PeerInfo {
        peer_id,
        address,
        user_agent: user_agent.clone(),
        protocol_version: 70015,
        services: services::NODE_NETWORK,
        start_height: 50000,
        relay_transactions: true,
        connected_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
    };
    
    assert_eq!(peer_info.peer_id, peer_id);
    assert_eq!(peer_info.address, address);
    assert_eq!(peer_info.user_agent, user_agent);
    assert_eq!(peer_info.protocol_version, 70015);
    assert_eq!(peer_info.services, services::NODE_NETWORK);
    
    println!("✓ PeerInfo structure validation passed");
}

#[tokio::test]
async fn test_transaction_broadcasting() {
    let config = NetworkConfig {
        listen_addr: "127.0.0.1:18444".parse().unwrap(),
        ..NetworkConfig::default()
    };
    
    let network_manager = NetworkManager::new(config).unwrap();
    let broadcaster = TransactionBroadcaster::new(std::sync::Arc::new(network_manager));
    
    // Create a test transaction
    // Create proper script signature for test
    let sig_addr = blockchain_core::script_utils::ScriptBuilder::generate_mining_address("network_test_sig");
    let sig_script = blockchain_core::script_utils::ScriptBuilder::create_coinbase_script(&sig_addr).unwrap();
    
    let tx_input = TransactionInput {
        previous_tx_hash: [1u8; 32],
        output_index: 0,
        script_sig: sig_script,
        sequence: 0xFFFFFFFF,
    };
    
    let tx_output = TransactionOutput::create_p2pkh(5000000, "test_address").unwrap();
    
    let transaction = Transaction {
        version: 1,
        inputs: vec![tx_input],
        outputs: vec![tx_output],
        lock_time: 0,
    };
    
    // Test transaction hash generation
    let tx_hash = transaction.get_hash().unwrap();
    assert_eq!(tx_hash.len(), 32);
    
    println!("✓ Transaction structure validation passed");
    
    // Test broadcast stats initialization
    let stats = broadcaster.get_stats().await;
    assert_eq!(stats.total_broadcasted, 0);
    assert_eq!(stats.pending_count, 0);
    assert_eq!(stats.seen_count, 0);
    
    println!("✓ Transaction broadcaster validation passed");
}

#[tokio::test]
async fn test_network_statistics() {
    let config = NetworkConfig::default();
    let manager = NetworkManager::new(config).unwrap();
    
    let stats = manager.get_stats().await;
    
    // Initial statistics should be zero
    assert_eq!(stats.connected_peers, 0);
    assert_eq!(stats.outbound_connections, 0);
    assert_eq!(stats.inbound_connections, 0);
    assert_eq!(stats.bytes_sent, 0);
    assert_eq!(stats.bytes_received, 0);
    assert_eq!(stats.messages_sent, 0);
    assert_eq!(stats.messages_received, 0);
    
    println!("✓ Network statistics validation passed");
}

#[tokio::test]
async fn test_address_manager() {
    let dns_seeds = vec!["localhost".to_string(), "example.com".to_string()];
    let address_manager = AddressManager::new(dns_seeds);
    
    // Test manual address addition
    let test_addr: SocketAddr = "192.168.1.50:8333".parse().unwrap();
    let result = address_manager.add_manual_address(test_addr, services::NODE_NETWORK).await;
    assert!(result.is_ok());
    
    // Test address distribution (should be empty initially)
    let distribution = address_manager.get_address_distribution().await;
    assert!(distribution.contains_key("manual"));
    
    println!("✓ AddressManager validation passed");
}

#[tokio::test]
async fn test_message_validation() {
    let ping_msg = Message::ping();
    
    // Test message validation
    assert!(ping_msg.validate());
    
    // Test checksum calculation
    let checksum = ping_msg.calculate_checksum();
    assert_ne!(checksum, 0);
    
    println!("✓ Message validation passed");
}

#[tokio::test]  
async fn test_protocol_constants() {
    use blockchain_network::protocol::{MAINNET_MAGIC, TESTNET_MAGIC, MAX_PAYLOAD_SIZE};
    
    assert_eq!(MAINNET_MAGIC, 0xD9B4BEF9);
    assert_eq!(TESTNET_MAGIC, 0x0709110B);
    assert_eq!(MAX_PAYLOAD_SIZE, 32 * 1024 * 1024);
    
    println!("✓ Protocol constants validation passed");
}

#[tokio::test]
async fn test_service_flags() {
    assert_eq!(services::NODE_NETWORK, 1 << 0);
    assert_eq!(services::NODE_UTXO, 1 << 1);
    assert_eq!(services::NODE_BLOOM, 1 << 2);
    assert_eq!(services::NODE_WITNESS, 1 << 3);
    assert_eq!(services::NODE_COMPACT_FILTERS, 1 << 6);
    assert_eq!(services::NODE_NETWORK_LIMITED, 1 << 10);
    
    // Test service combinations
    let combined_services = services::NODE_NETWORK | services::NODE_UTXO | services::NODE_WITNESS;
    assert_eq!(combined_services, 11); // 1 + 2 + 8 = 11
    
    println!("✓ Service flags validation passed");
}

async fn performance_benchmark() {
    println!("Running P2P network performance benchmark...");
    
    // Benchmark message creation
    let start = std::time::Instant::now();
    let mut messages = Vec::with_capacity(10000);
    
    for _i in 0..10000 {
        let msg = Message::ping();
        messages.push(msg);
    }
    
    let creation_time = start.elapsed();
    println!("Created 10,000 ping messages in {:?}", creation_time);
    println!("Rate: {:.0} messages/second", 10000.0 / creation_time.as_secs_f64());
    
    // Benchmark serialization
    let start = std::time::Instant::now();
    let mut serialized_data = Vec::with_capacity(10000);
    
    for msg in &messages[..1000] { // Test with first 1000 messages
        if let Ok(data) = msg.serialize() {
            serialized_data.push(data);
        }
    }
    
    let serialization_time = start.elapsed();
    println!("Serialized 1,000 messages in {:?}", serialization_time);
    println!("Serialization rate: {:.0} messages/second", 1000.0 / serialization_time.as_secs_f64());
    
    // Benchmark deserialization
    let start = std::time::Instant::now();
    let mut deserialized_count = 0;
    
    for data in &serialized_data {
        if Message::deserialize(data).is_ok() {
            deserialized_count += 1;
        }
    }
    
    let deserialization_time = start.elapsed();
    println!("Deserialized {} messages in {:?}", deserialized_count, deserialization_time);
    println!("Deserialization rate: {:.0} messages/second", deserialized_count as f64 / deserialization_time.as_secs_f64());
    
    println!("Performance benchmark completed!");
}

#[tokio::main]
async fn main() {
    println!("=== P2P Network Test Suite ===\n");
    
    // Run all tests
    let tests = vec![
        ("Network Config", test_network_config()),
        ("Network Manager Creation", test_network_manager_creation()),
        ("Protocol Messages", test_protocol_messages()),
        ("Network Address", test_network_address()),
        ("Peer Info", test_peer_info()),
        ("Transaction Broadcasting", test_transaction_broadcasting()),
        ("Network Statistics", test_network_statistics()),
        ("Address Manager", test_address_manager()),
        ("Message Validation", test_message_validation()),
        ("Protocol Constants", test_protocol_constants()),
        ("Service Flags", test_service_flags()),
    ];
    
    let mut passed = 0;
    let total = tests.len();
    
    for (name, test_future) in tests {
        match timeout(Duration::from_secs(10), test_future).await {
            Ok(_) => {
                println!("✅ {} - PASSED", name);
                passed += 1;
            }
            Err(_) => {
                println!("❌ {} - TIMEOUT", name);
            }
        }
    }
    
    // Run performance benchmark
    performance_benchmark().await;
    
    println!("\n=== Test Results ===");
    println!("Passed: {}/{}", passed, total);
    
    if passed == total {
        println!("🎉 All P2P network tests passed!");
        
        println!("\n📊 P2P Network Features:");
        println!("  • Async peer connection management");
        println!("  • Bitcoin-compatible protocol messages");
        println!("  • Automatic peer discovery via DNS seeds");
        println!("  • Transaction broadcasting with priority");
        println!("  • Network statistics and monitoring");
        println!("  • IPv4/IPv6 address support");
        println!("  • Message serialization/deserialization");
        println!("  • Service capability negotiation");
        println!("  • Connection timeout and heartbeat");
        println!("  • Comprehensive error handling");
    } else {
        println!("❌ Some tests failed");
    }
}