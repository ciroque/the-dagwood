# DAGwood Demo Setup Guide

This guide will help you set up and run the interactive DAGwood demonstration.

## Prerequisites

### 1. Rust Development Environment
Ensure you have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. mdBook Installation
Install mdBook and the Mermaid plugin for diagram rendering:
```bash
cargo install mdbook
cargo install mdbook-mermaid
```

### 3. Project Dependencies
Make sure all DAGwood dependencies are installed:
```bash
cd /data/development/projects/the-dagwood
cargo build --release
```

## Demo Components

### Interactive Presentation
The demo consists of:
- **mdBook presentation**: Located in `docs/demo/`
- **Progressive configurations**: 5 demo configs showing increasing complexity
- **Guided execution**: Modified `src/main.rs` with presentation prompts
- **Live examples**: Real DAG execution with WASM integration

### Demo Configurations
1. **01-hello-world.yaml**: Single processor introduction
2. **02-text-pipeline.yaml**: Linear chain showing data flow
3. **03-diamond-analysis.yaml**: Parallel execution with multiple strategies
4. **04-wasm-integration.yaml**: WASM processor sandboxing
5. **05-complex-workflow.yaml**: Advanced multi-backend pipeline

## Running the Demo

### 1. Build the Presentation
```bash
cd docs/demo
mdbook build
mdbook serve --open
```
This opens the interactive presentation in your browser at `http://localhost:3000`

### 2. Run the Interactive Demo
In a separate terminal:
```bash
cd /data/development/projects/the-dagwood
cargo run --release -- --demo-mode
```

The demo runner will:
- Show what each configuration demonstrates
- Wait for your keypress to proceed
- Execute the configuration with live output
- Explain the results and Rust concepts used

### 3. Presentation Flow (10-15 minutes)
1. **Introduction** (2 min): Project goals and architecture overview
2. **Hello World** (2 min): Basic processor and Rust concepts
3. **Text Pipeline** (3 min): DAG execution and data flow
4. **Diamond Analysis** (4 min): Parallel execution strategies
5. **WASM Integration** (3 min): Sandboxing and advanced backends
6. **Roadmap & Q&A** (2 min): Future plans and discussion

## Troubleshooting

### mdBook Issues
- **Port conflict**: Use `mdbook serve --port 3001` for different port
- **Build errors**: Ensure you're in `docs/demo/` directory

### Demo Execution Issues
- **Compilation errors**: Run `cargo clean && cargo build --release`
- **WASM module missing**: Ensure `wasm_modules/hello_world.wasm` exists
- **Config not found**: Verify you're running from project root directory

### Performance Tips
- **Pre-compile**: Run `cargo build --release` before presentation
- **Terminal setup**: Use large font and high contrast for visibility
- **Browser zoom**: Increase mdBook font size for audience visibility

## Demo Script Notes

### Key Talking Points
- **Rust Learning**: Highlight ownership, async/await, trait system usage
- **DAG Strategies**: Explain Work Queue vs Level-by-Level execution
- **WASM Benefits**: Security sandboxing and language flexibility
- **AI Assistance**: Mention how AI tools accelerated development

### Interactive Elements
- **Live coding**: Show configuration changes and immediate results
- **Performance comparison**: Demonstrate different execution strategies
- **Error handling**: Show graceful failure and recovery
- **Extensibility**: Add a new processor type during demo

## Backup Plans

### Technical Issues
- **Offline mode**: All examples work without internet
- **Static fallback**: Screenshots included in presentation
- **Manual execution**: Step-by-step commands provided

### Time Management
- **Short version** (10 min): Skip complex workflow demo
- **Extended version** (20 min): Add live coding segment
- **Q&A focus**: Emphasize interactive discussion

## Post-Demo Resources

### For Audience
- **GitHub repository**: Link to full source code
- **Documentation**: Point to ADRs and technical docs
- **Learning path**: Suggest starting points for exploration

### For Follow-up
- **Feedback collection**: Note questions and improvement areas
- **Demo evolution**: Update based on audience response
- **Technical deep-dives**: Schedule follow-up sessions for interested developers
