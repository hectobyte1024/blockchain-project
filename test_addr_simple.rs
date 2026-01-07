fn main() {
    let address = "edu1qTreasury00000000000000000000";
    let addr_bytes = address.as_bytes();
    
    let mut script = vec![0x76, 0xa9, addr_bytes.len() as u8];
    script.extend_from_slice(addr_bytes);
    script.extend_from_slice(&[0x88, 0xac]);
    
    println!("Original address: {}", address);
    println!("Script length: {}", script.len());
    
    // Extract
    if script.len() > 5 && script[0] == 0x76 && script[1] == 0xa9 {
        let addr_len = script[2] as usize;
        if script.len() >= 3 + addr_len + 2 {
            let address_bytes = &script[3..3+addr_len];
            let extracted = String::from_utf8(address_bytes.to_vec()).ok();
            println!("Extracted: {:?}", extracted);
            println!("Match: {}", extracted == Some(address.to_string()));
        }
    }
}
