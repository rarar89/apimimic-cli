<p align="center">
  <img src="https://apimimic.com/logo-dark.png" alt="Apimimic Logo" width="300"/>
</p>


# Apimimic CLI

A command-line interface tool for API mocking and proxying using [Apimimic](https://apimimic.com). Apimimic is a powerful API mocking platform that allows you to create, manage, and simulate API endpoints with ease. This CLI tool integrates with the Apimimic service to provide local API mocking and proxying capabilities.

## What is Apimimic?

Apimimic is a comprehensive API mocking solution that offers:

- 🚀 Fast and intuitive API mocking through a user-friendly interface
- 🔀 Proxy mode to selectively mock endpoints while forwarding others to your real API
- 🤖 AI-powered response generation
- ⚡ Automatic CRUD operation generation
- 📚 OpenAPI specification support

The CLI tool extends these capabilities to your local development environment, allowing you to:

- Intercept HTTP requests and return mocked JSON responses from your Apimimic project
- Forward unmocked requests to your actual backend when using proxy mode
- Seamlessly integrate with your development workflow
- Configure listening address and remote API endpoint address

## Installation

### Option 1: Download Release Binary (Recommended)

Download the latest release for your platform (Windows, macOS, or Linux) from our [GitHub Releases page](https://github.com/rarar89/apimimic-cli/releases).


### Option 2: Build from Source

To build from source, you'll need to have Rust and Cargo installed on your system:

```bash
git clone https://github.com/rarar89/apimimic-cli.git
cd apimimic-cli
cargo build --release
```

The binary will be available in `target/release/`.

## Usage

### Setting Project Key

Project key can be found in Apimimic dashboard in project view.

  <img src="https://apimimic.com/project-key-example-dark.jpg" alt="Apimimic Logo" width="400"/>


You can pre-set your project key by running:

```bash
apimimic set-project YOUR_PROJECT_KEY
```

Or provide it directly when running the server:

```bash
apimimic run --project YOUR_PROJECT_KEY
```

### Starting the Server

Basic usage (starts server on default port 8080):
```bash
apimimic run
```

With custom configuration:
```bash
apimimic run --listen 127.0.0.1:3000 --project YOUR_PROJECT_KEY --server http://localhost:3001
```

### Command Line Options

- `help`: Show help message
- `set-project <key>`: Save the project KEY
- `run`: Start the HTTP server with the following options:
  - `-p, --project <key>`: Project KEY. Required if not set with `apimimic set-project`
  - `-l, --listen <address>`: Local address to listen on (default: 127.0.0.1:8080)
  - `-r, --remote <url>`: Remote API Mimic URL (default: https://cli.apimimic.com)
  - `--server <url>`: real api server URL (required if proxy mode is enabled on apimimic.com)

## Configuration

The tool stores configuration in the following location:
- Windows: `%APPDATA%\apimimic\apimimic-cli\config\config.json`
- macOS: `~/Library/Application Support/com.apimimic.apimimic-cli/config.json`
- Linux: `~/.config/apimimic-cli/config.json`

## How It Works

1. Create and configure your mock API endpoints through the Apimimic web interface (https://apimimic.com)
2. Use the CLI tool to start a local server that connects to your Apimimic project
3. Direct your application's API requests to the CLI server
4. The CLI tool will:
   - Forward requests to the Apimimic service
   - Return mocked responses for configured endpoints
   - Proxy unmocked requests to your real backend (when proxy mode is enabled)

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
