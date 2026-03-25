use anyhow::Result;
use crate::context::Context;

pub fn run(verbose: bool) -> Result<()> {
    if verbose {
        println!("→ Setting up project environment...");
    }

    // Load context (this implicitly sets up the project structure if needed)
    let ctx = Context::new(verbose)?;

    // Resolve and install Typst toolchain based on typstlab.toml
    let resolve_options = typstlab_typst::resolve::ResolveOptions {
        required_version: ctx.config.typst.version.clone(),
        project_root: ctx.project.root.clone(),
        force_refresh: true, // "毎回installしなおします" に対応
    };

    if verbose {
        println!("→ Resolving Typst v{}...", ctx.config.typst.version);
    }
    
    let result = typstlab_typst::resolve::resolve_typst(resolve_options)?;
    
    match result {
        typstlab_typst::resolve::ResolveResult::Resolved(info) |
        typstlab_typst::resolve::ResolveResult::Cached(info) => {
            if verbose {
                println!("✓ Typst v{} resolved at {}", info.version, info.path.display());
            }
            
            // Create shim and update state (same as typst install)
            crate::commands::typst::util::create_bin_shim(&ctx.project.root, &info.path)?;
            crate::commands::typst::util::update_state(
                &ctx.project.root, 
                &info.path, 
                &info.version, 
                info.source.to_string()
            )?;

            if verbose {
                println!("✓ Setup complete. Currently active Typst path: {}", info.path.display());
            }
        }
        typstlab_typst::resolve::ResolveResult::NotFound { required_version, .. } => {
            anyhow::bail!("Failed to setup: Typst {} not found after resolution attempt.", required_version);
        }
    }

    Ok(())
}
