# API Mimic CLI

A command-line interface tool for API mocking and proxying using [API Mimic](https://apimimic.com). This tool allows you to intercept HTTP requests and either return mocked responses from API Mimic service or proxy them to a local backend.

## Features

- 🔄 Mock API responses using API Mimic service
- 🔀 Proxy mode for forwarding requests to local backend
- 🔐 Token-based authentication
- 🌐 Configurable listening address and remote API endpoint
- 💾 Persistent token storage

## Installation

To install the API Mimic CLI, you'll need to have Rust and Cargo installed on your system. Then you can build the project:

```bash
cargo build --release
```

The binary will be available in `target/release/`.

## Usage

### Setting Authentication Token

Before using the tool, you need to set your API Mimic authentication token:

```bash
apimimic-cli set-token YOUR_TOKEN_HERE
```

Or provide it directly when running the server:

```bash
apimimic-cli run --token YOUR_TOKEN_HERE
```

### Starting the Server

Basic usage (starts server on default port 8080):
```bash
apimimic-cli run
```

With custom configuration:
```bash
apimimic-cli run --listen 127.0.0.1:3000
```

### Proxy Mode

To enable proxy mode (forwarding requests to a local backend):
```bash
apimimic-cli run --proxy --listen 127.0.0.1:3000 --backend http://localhost:3001
```

### Command Line Options

- `help`: Show help message
- `set-token <token>`: Save the authentication token
- `run`: Start the HTTP server with the following options:
  - `-l, --listen <address>`: Local address to listen on (default: 0.0.0.0:8080)
  - `-r, --remote <url>`: Remote API Mimic URL (default: https://cli.apimimic.com)
  - `-t, --token <token>`: Authorization token (overrides saved token)
  - `--proxy`: Enable proxy mode
  - `--backend <url>`: Local backend URL (required if proxy mode is enabled)

## Configuration

The tool stores configuration (including your authentication token) in the following location:
- Windows: `%APPDATA%\apimimic\apimimic-cli\config\config.json`
- macOS: `~/Library/Application Support/com.apimimic.apimimic-cli/config.json`
- Linux: `~/.config/apimimic-cli/config.json`

## How It Works

1. The tool starts an HTTP server on the specified address
2. For each incoming request:
   - Forwards the request to the API Mimic service
   - If proxy mode is enabled and the API Mimic response indicates proxying:
     - Forwards the original request to the specified backend
   - Otherwise, returns the response from API Mimic

## Error Handling

The tool includes comprehensive error handling for:
- Missing authentication token
- Failed connections to API Mimic service
- Failed connections to local backend
- Invalid configuration
- Request/response processing errors

## License

[Add your license information here] 