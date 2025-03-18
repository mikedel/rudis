# Rudis - A Rust Redis Alternative

Rudis is a high-performance key-value store written in Rust, designed as a lightweight Redis alternative. It focuses on core Redis functionality with a simpler implementation while maintaining compatibility with Redis clients.

## Features

- Key-value storage with optional expiration (TTL)
- FIFO queue operations (POP command)
- Redis protocol compatibility
- High-performance concurrent access using DashMap
- Simple, focused API

## Performance

Rudis is designed for high performance:
- Lock-free concurrent access
- Efficient memory management with Bytes
- Asynchronous I/O with Tokio
- Minimal overhead and dependencies

## Getting Started

### Local Setup

1. Clone the repository:
```bash
git clone https://github.com/mikedel/rudis.git
```

2. Build the project:
```bash
cargo build
```

3. Run the server:
```bash
cargo run --bin rudis
```

4. Test the server:
```bash
cargo run --bin benchmark
```