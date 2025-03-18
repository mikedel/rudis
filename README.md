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

### Installation

```bash
cargo install rudis