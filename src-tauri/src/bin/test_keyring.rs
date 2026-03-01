use keyring::Entry;

fn main() {
    let service = "coder-ai";
    let username = "test-provider-store";
    
    println!("Testing keyring with service: {}, username: {}", service, username);
    
    // Create entry
    let entry = match Entry::new(service, username) {
        Ok(e) => {
            println!("Entry created successfully");
            e
        },
        Err(e) => {
            eprintln!("Failed to create entry: {}", e);
            std::process::exit(1);
        }
    };
    
    // Set password
    match entry.set_password("test-api-key-12345") {
        Ok(_) => println!("Password set successfully"),
        Err(e) => {
            eprintln!("Failed to set password: {}", e);
            std::process::exit(1);
        }
    }
    
    // Get password
    match entry.get_password() {
        Ok(pw) => println!("Password retrieved: {}", pw),
        Err(e) => {
            eprintln!("Failed to get password: {}", e);
            std::process::exit(1);
        }
    }
    
    // Delete password
    match entry.delete_credential() {
        Ok(_) => println!("Password deleted successfully"),
        Err(e) => println!("Failed to delete password: {}", e),
    }
    
    println!("All tests passed!");
}
