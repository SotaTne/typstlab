use anyhow::Result;

pub fn run(verbose: bool) -> Result<()> {
    if verbose {
        println!("→ Setting up project environment...");
    }

    // Setup is equivalent to sync --all
    crate::commands::sync::run(false, false, true, verbose)?;

    if verbose {
        println!("✓ Setup complete");
    }

    Ok(())
}
