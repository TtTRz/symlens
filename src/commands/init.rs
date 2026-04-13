use crate::config;

pub fn run(root_override: Option<&str>) -> anyhow::Result<()> {
    let root = crate::commands::resolve_root(root_override)?;
    let target = root.join("codelens.toml");

    if target.exists() {
        anyhow::bail!(
            "codelens.toml already exists at {}. Use --force or delete it first.",
            target.display()
        );
    }

    std::fs::write(&target, config::default_toml())?;
    println!("Created {}", target.display());
    println!("Edit this file to customize indexing behavior.");
    Ok(())
}
