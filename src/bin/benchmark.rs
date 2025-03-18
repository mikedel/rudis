use std::time::{Duration, Instant};
use bytes::Bytes;
use clap::Parser;
use tokio::time;

// Import client from main crate
use rudis::client::Client;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Redis server address
    #[arg(short, long, default_value = "127.0.0.1:6379")]
    address: String,
    
    /// Number of operations to perform
    #[arg(short, long, default_value = "10000")]
    operations: usize,
    
    /// Value size in bytes
    #[arg(short, long, default_value = "100")]
    value_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("Connecting to {}", args.address);
    let mut client = Client::connect(&args.address).await?;
    
    // Generate test data
    let value = vec![b'x'; args.value_size];
    
    // Benchmark SET operations
    println!("Benchmarking SET operations...");
    let start = Instant::now();
    
    for i in 0..args.operations {
        let key = format!("bench:key:{}", i);
        client.set(&key, &value, None).await?;
    }
    
    let elapsed = start.elapsed();
    let ops_per_sec = args.operations as f64 / elapsed.as_secs_f64();
    
    println!("SET: {} operations in {:?} ({:.2} ops/sec)", 
        args.operations, elapsed, ops_per_sec);
    
    // Benchmark GET operations
    println!("Benchmarking GET operations...");
    let start = Instant::now();
    
    for i in 0..args.operations {
        let key = format!("bench:key:{}", i);
        let _ = client.get(&key).await?;
    }
    
    let elapsed = start.elapsed();
    let ops_per_sec = args.operations as f64 / elapsed.as_secs_f64();
    
    println!("GET: {} operations in {:?} ({:.2} ops/sec)", 
        args.operations, elapsed, ops_per_sec);
    
    Ok(())
}