use env_logger::Env;

/// Initialize a colorful logger with emojis and a light startup banner.
pub fn init() {
    let _env = Env::default().filter_or("RUST_LOG", "info");

    // Defer global logger initialization to the CLI/runner (some runtime
    // components initialize logging themselves). We avoid calling
    // `env_logger`/`tracing_subscriber` here to prevent double-initialization
    // which causes the node to fail at startup.

    // NOTE: if you need logging locally, enable it via the usual RUST_LOG env
    // or let the CLI/runner initialize logging.

    // Defer logging setup entirely to the CLI/runner. The runner initializes
    // logging and tracing subscribers in the correct order to avoid conflicts.

    // Colorful startup banner with ASCII art (ANSI color) — visible even if
    // logger is overridden.
    //
    // On **stderr**: stdout is the data channel for the subcommands whose output
    // is meant to be piped — `build-spec > spec.json`, `export-state`,
    // `export-blocks`, `keys`. Printing the banner to stdout put the ASCII art
    // in front of every generated chain spec, so the file started
    // `{\n\n<esc>[1;35m` and no JSON parser would take it. Every spec in
    // `chain-specs/` was produced by this command.
    eprintln!("\n\x1b[1;35m");
    eprintln!("       ________          __                ");
    eprintln!("___  __\\_____  \\  ______/  |______ _______ ");
    eprintln!("\\  \\/  / _(__  < /  ___|   __\\__  \\\\_  __ \\");
    eprintln!(r" >    < /       \\\___ \ |  |  / __ \|  | \/");
    eprintln!("/__/\\_Y______  /____  >|__| (____  /__|   ");
    eprintln!("     \\/      \\/     \\/           \\/       ");
    eprintln!("\x1b[0m\x1b[36m\n🚀  X3 Chain Node — syncing the mesh ⚡️\x1b[0m\n");
}
