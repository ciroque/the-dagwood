# ADR 025: Single Binary with Subcommands

## Status
Proposed

## Context

The DAGwood project is evolving from a library-focused tool to support multiple use cases:
- **Library usage**: Embed DAGwood in applications for programmatic pipeline execution
- **CLI tool**: Execute pipelines from command line for testing and automation
- **Daemon/server**: Long-running process hosting multiple pipelines with protocol receivers
- **Validation tool**: Check pipeline configurations without executing
- **Inspection tool**: Analyze pipeline structure, dependencies, execution order

Different use cases have different requirements:
- **Library**: No binary needed, just Rust crate
- **CLI**: Single execution, exit after completion
- **Daemon**: Long-running, signal handling, graceful shutdown
- **Validation**: Quick check, exit with status code
- **Inspection**: Display information, exit

**Decision needed**: How should we structure the binary/binaries to support these use cases while maintaining simplicity?

## Decision

We will provide a **Single Binary with Subcommands** pattern, similar to `cargo`, `git`, and `docker`.

### Binary Structure

```
dagwood <subcommand> [options]
```

### Subcommands

**dagwood run**
- Execute pipeline once with input, exit after completion
- Current behavior (backward compatible)
- Use case: CLI tool, testing, automation scripts

```bash
dagwood run --config pipeline.yaml --input input.txt
dagwood run --config pipeline.yaml --input-stdin < input.txt
```

**dagwood serve**
- Start daemon with protocol receivers
- Long-running process
- Use case: Production server, multi-pipeline hosting

```bash
dagwood serve --config server.yaml
dagwood serve --config server.yaml --watch  # Auto-reload on config change
```

**dagwood validate**
- Validate configuration without executing
- Check syntax, dependencies, processor availability
- Exit with status code (0 = valid, 1 = invalid)
- Use case: CI/CD pipelines, pre-deployment checks

```bash
dagwood validate --config pipeline.yaml
dagwood validate --config server.yaml
```

**dagwood inspect**
- Display pipeline structure and metadata
- Show dependency graph, execution order, processor details
- Use case: Debugging, documentation, understanding pipelines

```bash
dagwood inspect --config pipeline.yaml
dagwood inspect --config pipeline.yaml --format json
dagwood inspect --config pipeline.yaml --graph  # Show ASCII dependency graph
```

### Configuration Compatibility

**Legacy format (single pipeline):**
```yaml
strategy: work_queue
processors: [...]
```

Works with:
- `dagwood run` - Execute this pipeline
- `dagwood validate` - Validate this pipeline
- `dagwood inspect` - Inspect this pipeline
- `dagwood serve` - Serve as "default" pipeline

**Server format (multiple pipelines):**
```yaml
protocols: [...]
pipelines:
  - name: pipeline1
  - name: pipeline2
```

Works with:
- `dagwood serve` - Start server with all pipelines
- `dagwood validate` - Validate all pipelines
- `dagwood inspect --pipeline pipeline1` - Inspect specific pipeline

## Alternatives Considered

### Alternative 1: Separate Binaries

**Approach**: `dagwood`, `dagwood-server`, `dagwood-validate`, `dagwood-inspect`

**Pros**:
- Smallest binaries: Only include code for specific use case
- Clear separation: Each binary has single purpose
- Independent versioning: Could version separately

**Cons**:
- Deployment complexity: Multiple binaries to distribute
- User confusion: Which binary to use?
- Code duplication: Shared logic repeated across binaries
- Build complexity: Multiple build targets
- Installation complexity: Multiple binaries to install

**Rejected**: Operational complexity outweighs binary size benefits. Single binary with subcommands is more user-friendly.

### Alternative 2: Mode Detection (No Subcommands)

**Approach**: Detect mode from flags or config

```bash
dagwood --config pipeline.yaml --input input.txt  # Auto-detect: run mode
dagwood --config server.yaml  # Auto-detect: serve mode (has protocols)
dagwood --config pipeline.yaml --validate  # Validate mode
```

**Pros**:
- Simpler CLI: No subcommands needed
- Backward compatible: Existing commands work unchanged
- Less typing: Shorter commands

**Cons**:
- Ambiguous: Hard to tell what command will do
- Magic behavior: Mode detection is implicit, not explicit
- Confusing: `--validate` flag vs `validate` subcommand
- Error-prone: Easy to accidentally run wrong mode
- Inconsistent: Different from common CLI patterns (cargo, git, docker)

**Rejected**: Implicit behavior is confusing. Explicit subcommands make intent clear.

### Alternative 3: Library Only (No Binary)

**Approach**: Provide only Rust crate, users build their own binaries

**Pros**:
- Maximum flexibility: Users can customize binary
- No CLI maintenance: Users handle their own CLI
- Smaller scope: Focus on library functionality

**Cons**:
- Poor UX: Users must write boilerplate for common tasks
- Barrier to entry: Requires Rust knowledge to use
- Fragmentation: Every user builds different CLI
- No standard tool: Cannot share scripts or documentation

**Rejected**: Binary is essential for usability. Most users want ready-to-use tool, not library.

### Alternative 4: Plugin System

**Approach**: Core binary with plugin subcommands

```bash
dagwood run  # Built-in
dagwood serve  # Built-in
dagwood my-custom-command  # Plugin
```

**Pros**:
- Extensible: Users can add custom subcommands
- Community contributions: Third-party plugins
- Flexible: Customize without forking

**Cons**:
- Complex: Plugin system is substantial engineering
- Security: Plugins could be malicious
- Stability: Plugin API must be stable
- Overkill: No clear need for plugins yet

**Rejected**: Premature. Can add plugin system later if demand emerges.

## Consequences

### Positive

- **Clear Intent**: Subcommand makes purpose explicit
- **Familiar Pattern**: Matches cargo, git, docker, kubectl
- **Single Binary**: Easy to distribute and install
- **Backward Compatible**: `dagwood run` matches current behavior
- **Extensible**: Easy to add new subcommands later
- **Help Text**: Each subcommand can have specific help
- **Consistent UX**: Same patterns across all subcommands

### Negative

- **Slightly Longer Commands**: `dagwood run` vs `dagwood`
- **Learning Curve**: Users must learn subcommands (but familiar pattern)
- **Binary Size**: Includes all subcommands (but not significant)

### Neutral

- **Compilation Time**: Single binary compiles all code (same as before)
- **Testing**: Need to test each subcommand (but would need to anyway)

## Implementation Notes

### Phase 2.3: dagwood serve Subcommand

**Initial implementation:**
- Add CLI subcommand parsing using `clap`
- Implement `dagwood serve --config <path>` command
- Load server config (protocols + pipelines)
- Start all configured protocol receivers
- Implement graceful shutdown on Ctrl+C

See [DAEMONIZATION_ROADMAP.md - Phase 2.3](../../DAEMONIZATION_ROADMAP.md#23-dagwood-serve-subcommand-) for detailed implementation plan.

**Future subcommands:**
- `dagwood validate` - Phase 5 or later
- `dagwood inspect` - Phase 5 or later

### CLI Structure

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dagwood")]
#[command(about = "DAG-based workflow orchestration engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute pipeline once and exit
    Run {
        #[arg(short, long)]
        config: PathBuf,
        
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
    
    /// Start server with protocol receivers
    Serve {
        #[arg(short, long)]
        config: PathBuf,
        
        #[arg(short, long)]
        watch: bool,
    },
    
    /// Validate configuration
    Validate {
        #[arg(short, long)]
        config: PathBuf,
    },
    
    /// Inspect pipeline structure
    Inspect {
        #[arg(short, long)]
        config: PathBuf,
        
        #[arg(short, long)]
        pipeline: Option<String>,
    },
}
```

See [DAEMONIZATION_ROADMAP.md - Phase 2.3](../../DAEMONIZATION_ROADMAP.md#23-dagwood-serve-subcommand-) for detailed implementation plan.

## Related ADRs

- [ADR 20 - Multi-Pipeline Architecture & Registry Pattern](./ADR%2020%20-%20Multi-Pipeline%20Architecture%20&%20Registry%20Pattern.md) - `serve` subcommand hosts multiple pipelines
- [ADR 21 - Pluggable Protocol Receiver Architecture](./ADR%2021%20-%20Pluggable%20Protocol%20Receiver%20Architecture.md) - `serve` subcommand starts protocol receivers
- [ADR 23 - Hot-Reload Strategy](./ADR%2023%20-%20Hot-Reload%20Strategy%20(Drain-and-Switch).md) - `serve --watch` enables hot-reload

## References

- [DAEMONIZATION_ROADMAP.md - Phase 2.3: dagwood serve Subcommand](../../DAEMONIZATION_ROADMAP.md#23-dagwood-serve-subcommand-)
- clap Documentation: https://docs.rs/clap/
- CLI Design Patterns: https://clig.dev/
