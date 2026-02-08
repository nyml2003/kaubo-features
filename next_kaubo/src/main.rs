mod compiler;
mod kit;

use std::{fs, thread::sleep, time::Duration};

fn main() {
    let path = r"C:\Users\nyml\code\kaubo-features\next_kaubo\assets\a.txt";
    let content = fs::read_to_string(path).expect("Failed to read file");
    
    let tokens = next_kaubo::tokenize(&content);
    
    for token in tokens {
        println!("{:?}", token);
        sleep(Duration::from_millis(100));
    }
}
