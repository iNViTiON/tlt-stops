# TLT Stop Tracker

## Live Application

The application is live and accessible at [https://tlt.invition.dev](https://tlt.invition.dev).

## Overview
TLT Stops is a modern web application designed to provide real-time transit information. The project is built with a focus on performance, scalability, developer experince, and user experience. It leverages cutting-edge technologies such as Rust, WebAssembly (WASM), and Svelte for the frontend, and is designed to run on Cloudflare Workers for a serverless, globally distributed backend.

## Repository Structure
The repository is organized as follows:

```
.
├── frontend/               # Svelte-based frontend
│   ├── src/                # Svelte components and TypeScript files
│   ├── public/             # Static assets
│   ├── package.json        # Frontend dependencies
│   ├── vite.config.ts      # Vite configuration for the frontend
│   ├── tsconfig.json       # TypeScript configuration
│   └── README.md           # Frontend-specific documentation
├── src/                    # Rust backend source files
├── Cargo.toml              # Rust project configuration
├── package.json            # Root dependencies
├── wrangler.toml           # Cloudflare Workers configuration
└── README.md               # Project documentation
```

## Technologies Used

### Backend
- **Language**: Rust
- **Platform**: Cloudflare Workers
- **Build Tool**: Wrangler CLI
- **WebAssembly**: Used for high-performance server-side logic

### Frontend
- **Framework**: Svelte
- **Build Tool**: Vite
- **Language**: TypeScript

### Package Manager
- **Bun.js**: Used exclusively for managing dependencies and running scripts

## Installation

### Prerequisites
- Install [Bun.js](https://bun.sh/)
- Install [Wrangler CLI](https://developers.cloudflare.com/workers/wrangler/)
- Install [Rust](https://www.rust-lang.org/tools/install)
- Install the `wasm32-unknown-unknown` target for Rust:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

### Steps
1. Clone the repository:
   ```bash
   git clone https://github.com/iNViTiON/tlt-stops.git
   cd tlt-stops
   ```
2. Install dependencies:
   ```bash
   bun install
   ```

## Development

### Backend
To run the backend locally:
```bash
bunx wrangler dev
```

### Frontend
To run the frontend locally:
```bash
cd frontend
bun run dev
```

## Best Practices

### Documentation
- Keep the `README.md` file updated with any changes to the project structure or setup.
- Use inline comments in code to explain complex logic.

### Code Quality
- Follow Rust's idiomatic practices for the backend.
- Use TypeScript for type safety in the frontend.
- Run linters and formatters before committing code.

### Performance
- Optimize WebAssembly modules for faster execution.
- Use Vite's production build for deploying the frontend.

### Deployment
- Use the following build command in `wrangler.toml` for the fastest startup during development, after running the default one at least one time or manually run `cargo install worker-build`:
  ```toml
  [build]
  command = "worker-build --dev"
  ```
- Deploy the backend using Wrangler:
  ```bash
  bunx wrangler deploy
  ```

## Contributing
Contributions are welcome! Please follow these steps:
1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Submit a pull request with a detailed description of your changes.

## License
This project is licensed under the MIT License. See the `LICENSE` file for details.

---

For more information, refer to the [Cloudflare Workers documentation](https://developers.cloudflare.com/workers/).
