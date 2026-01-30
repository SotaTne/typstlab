use anyhow::Result;

pub fn run(name: String, _verbose: bool) -> Result<()> {
    println!("Creating library '{}'...", name);
    println!("! gen lib is not yet implemented (Coming in v0.2)");
    Ok(())
}
