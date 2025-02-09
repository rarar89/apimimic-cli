# Apimimic CLI

A command-line interface tool for API mocking and proxying using [Apimimic](https://apimimic.com). This tool allows you to intercept HTTP requests and either return mocked responses from Apimimic service or proxy unmocked requests to a local backend.

## Features

- 🔄 Mock API responses using Apimimic service
- 🔀 Proxy mode for forwarding unmocked requests to local backend
- 🔐 Token-based authentication
- 🌐 Configurable listening address and remote API endpoint

## Installation

To install the Apimimic CLI, you'll need to have Rust and Cargo installed on your system. Then you can build the project:

```bash
cargo build --release
```

The binary will be available in `target/release/`.

## Usage

### Setting Authentication Token

Before using the tool, you need to set your Apimimic authentication token and project ID:

```bash
apimimic set-token YOUR_TOKEN_HERE
```

```bash
apimimic set-project YOUR_PROJECT_ID
```

Or provide it directly when running the server:

```bash
apimimic run --token YOUR_TOKEN_HERE --project YOUR_PROJECT_ID
```

### Starting the Server

Basic usage (starts server on default port 8080):
```bash
apimimic run
```

With custom configuration:
```bash
apimimic run --listen 127.0.0.1:3000 --project YOUR_PROJECT_ID --backend http://localhost:3001
```

### Command Line Options

- `help`: Show help message
- `set-token <token>`: Save the authentication token
- `run`: Start the HTTP server with the following options:
  - `-l, --listen <address>`: Local address to listen on (default: 0.0.0.0:8080)
  - `-r, --remote <url>`: Remote API Mimic URL (default: https://cli.apimimic.com)
  - `-t, --token <token>`: Authorization token (overrides saved token)
  - `-p, --project <id>`: Project ID (required)
  - `--backend <url>`: Local backend URL (required if proxy mode is enabled on apimimic.com)

## Configuration

The tool stores configuration (including your authentication token) in the following location:
- Windows: `%APPDATA%\apimimic\apimimic-cli\config\config.json`
- macOS: `~/Library/Application Support/com.apimimic.apimimic-cli/config.json`
- Linux: `~/.config/apimimic-cli/config.json`

## How It Works

1. The tool starts an HTTP server on the specified address
2. For each incoming request:
   - Forwards the request to the Apimimic service
   - If proxy mode is enabled and the Apimimic response indicates proxying:
     - Forwards the original request to the specified backend
   - Otherwise, returns the response from Apimimic

## Error Handling

The tool includes comprehensive error handling for:
- Missing authentication token
- Failed connections to Apimimic service
- Failed connections to local backend
- Invalid configuration
- Request/response processing errors

## License

MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE. 
