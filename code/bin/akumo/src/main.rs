//! Akumo binary entrypoint. Deliberately thin: it wires adapters into the core and hands control to
//! the CLI driving adapter (ADR-0001). All real logic lives in the library crates so the core is
//! embeddable (FR-K3) and testable without the binary.

fn main() {
    let code = akumo_cli::run();
    std::process::exit(code);
}
