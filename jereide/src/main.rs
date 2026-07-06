#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
// Detach from the console on release Windows builds so launching
// jereide.exe from Explorer / a shortcut doesn't flash a black
// terminal window behind the editor. Debug builds keep the console so
// `eprintln!` output stays visible when developing.

use jereide_core::editor::subsystems::EditorSubsystems;

fn main() {
    env_logger::init();
    jereide_core::signal::install_handlers();
    let args: Vec<String> = std::env::args().collect();
    if let Err(e) = run(&args) {
        eprintln!("Fatal: {e:#}");
        std::process::exit(1);
    }
}

fn run(args: &[String]) -> anyhow::Result<()> {
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");

    jereide_core::window::init()?;

    let runtime = jereide_core::runtime::RuntimeContext::discover()?;
    let mut config = jereide_core::editor::config::NativeConfig::load_or_default(
        &runtime.user_dir_str(),
        runtime.scale(),
        runtime.platform_name(),
        &runtime.data_dir_str(),
    );
    config.verbose = verbose;

    let subsystems = EditorSubsystems::all();
    jereide_core::editor::main_loop::run(
        config,
        args,
        &runtime.data_dir_str(),
        &runtime.user_dir_str(),
        subsystems,
    );

    jereide_core::window::shutdown();

    Ok(())
}
