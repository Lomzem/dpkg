use std::path::Path;

use crate::config::parse_config;
use crate::error::DpkgError;
use crate::output;

pub fn run(config_path: &Path, quiet: bool) -> Result<(), DpkgError> {
    let config = parse_config(config_path)?;

    if !quiet {
        output::success(&format!(
            "Configuration is valid: {}",
            config_path.display()
        ));

        let total_packages: usize = config.sections.iter().map(|s| s.packages.len()).sum();
        output::plain(&format!(
            "  {} sections, {} total package entries",
            config.sections.len(),
            total_packages
        ));
    }

    Ok(())
}
