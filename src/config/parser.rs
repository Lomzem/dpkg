use std::path::Path;

use crate::config::types::{Config, Header, Package, PackageSource, Section};
use crate::error::DpkgError;

pub fn parse_config(path: &Path) -> Result<Config, DpkgError> {
    if !path.exists() {
        return Err(DpkgError::ConfigNotFound {
            path: path.to_path_buf(),
        });
    }
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            DpkgError::PermissionDenied(format!("Cannot read config file: {}", path.display()))
        } else {
            DpkgError::ConfigParse {
                line: 0,
                message: format!("Failed to read config file: {e}"),
            }
        }
    })?;
    parse_config_str(&content)
}

pub fn parse_config_str(input: &str) -> Result<Config, DpkgError> {
    let mut sections: Vec<Section> = Vec::new();

    for (line_num_0, raw_line) in input.lines().enumerate() {
        let line_num = line_num_0 + 1;

        // Strip comments: find first `//`, take everything before it
        let line = match raw_line.find("//") {
            Some(pos) => &raw_line[..pos],
            None => raw_line,
        };

        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Section header
        if line.starts_with("##") {
            let after_hashes = &line[2..];

            // Must have a space after ##
            if !after_hashes.starts_with(' ') {
                return Err(DpkgError::ConfigParse {
                    line: line_num,
                    message: format!(
                        "Invalid section header: `{line}`\n  Expected: ## * or ## @<hostname>\n  Hint: Section headers must have a space after ##"
                    ),
                });
            }

            let header_value = after_hashes[1..].trim();

            let header = if header_value == "*" {
                Header::All
            } else if let Some(hostname) = header_value.strip_prefix('@') {
                let hostname = hostname.trim();
                if hostname.is_empty() {
                    return Err(DpkgError::ConfigParse {
                        line: line_num,
                        message: "Empty hostname in section header".to_string(),
                    });
                }
                // Validate hostname: alphanumeric and hyphens
                if !hostname
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-')
                {
                    return Err(DpkgError::ConfigParse {
                        line: line_num,
                        message: format!(
                            "Invalid hostname `{hostname}`: only alphanumeric characters and hyphens are allowed"
                        ),
                    });
                }
                Header::Hostname(hostname.to_string())
            } else {
                return Err(DpkgError::ConfigParse {
                    line: line_num,
                    message: format!(
                        "Invalid section header: `{line}`\n  Expected: ## * or ## @<hostname>"
                    ),
                });
            };

            sections.push(Section {
                header,
                packages: Vec::new(),
            });
            continue;
        }

        // Package line — must be inside a section
        if sections.is_empty() {
            return Err(DpkgError::ConfigParse {
                line: line_num,
                message: "Package found before any section header".to_string(),
            });
        }

        let (name, source) = if let Some(aur_name) = line.strip_prefix("aur:") {
            let aur_name = aur_name.trim();
            if aur_name.is_empty() {
                return Err(DpkgError::ConfigParse {
                    line: line_num,
                    message: "Empty AUR package name after `aur:` prefix".to_string(),
                });
            }
            (aur_name.to_string(), PackageSource::Aur)
        } else {
            (line.to_string(), PackageSource::Official)
        };

        sections.last_mut().unwrap().packages.push(Package { name, source });
    }

    Ok(Config { sections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Header;

    #[test]
    fn test_parse_minimal() {
        let input = "## *\nbase\ngit\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].header, Header::All);
        assert_eq!(config.sections[0].packages.len(), 2);
        assert_eq!(config.sections[0].packages[0].name, "base");
        assert_eq!(config.sections[0].packages[1].name, "git");
    }

    #[test]
    fn test_parse_hostname_section() {
        let input = "## @MyHost\nnvidia\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(
            config.sections[0].header,
            Header::Hostname("MyHost".to_string())
        );
    }

    #[test]
    fn test_parse_aur_packages() {
        let input = "## *\naur:yay\naur:visual-studio-code-bin\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections[0].packages[0].name, "yay");
        assert_eq!(
            config.sections[0].packages[0].source,
            PackageSource::Aur
        );
        assert_eq!(
            config.sections[0].packages[1].name,
            "visual-studio-code-bin"
        );
    }

    #[test]
    fn test_parse_comments() {
        let input = "// standalone comment\n## * // section comment\nbase // inline comment\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections.len(), 1);
        assert_eq!(config.sections[0].packages.len(), 1);
        assert_eq!(config.sections[0].packages[0].name, "base");
    }

    #[test]
    fn test_parse_empty_lines() {
        let input = "\n\n## *\n\nbase\n\ngit\n\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections[0].packages.len(), 2);
    }

    #[test]
    fn test_parse_multiple_sections() {
        let input = "## *\nbase\n\n## @Desktop\nnvidia\n\n## @Laptop\ntlp\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections.len(), 3);
    }

    #[test]
    fn test_parse_invalid_header_no_space() {
        let input = "##*\nbase\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            DpkgError::ConfigParse { line, .. } => assert_eq!(line, 1),
            _ => panic!("Expected ConfigParse error"),
        }
    }

    #[test]
    fn test_parse_invalid_header_no_marker() {
        let input = "## something\nbase\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_hostname() {
        let input = "## @\nbase\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_aur_name() {
        let input = "## *\naur:\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_package_before_section() {
        let input = "base\n## *\ngit\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
        match result.unwrap_err() {
            DpkgError::ConfigParse { line, .. } => assert_eq!(line, 1),
            _ => panic!("Expected ConfigParse error"),
        }
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let input = "## *\n  base  \n\tgit\t\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections[0].packages[0].name, "base");
        assert_eq!(config.sections[0].packages[1].name, "git");
    }

    #[test]
    fn test_parse_empty_config() {
        let input = "";
        let config = parse_config_str(input).unwrap();
        assert!(config.sections.is_empty());
    }

    #[test]
    fn test_parse_only_comments() {
        let input = "// just a comment\n// another comment\n";
        let config = parse_config_str(input).unwrap();
        assert!(config.sections.is_empty());
    }

    #[test]
    fn test_parse_multiple_slashes_in_comment() {
        let input = "## *\nnvidia // uses // in path comments are fine // this is still a comment\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(config.sections[0].packages[0].name, "nvidia");
    }

    #[test]
    fn test_parse_hostname_with_hyphen() {
        let input = "## @my-desktop\nnvidia\n";
        let config = parse_config_str(input).unwrap();
        assert_eq!(
            config.sections[0].header,
            Header::Hostname("my-desktop".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_hostname_chars() {
        let input = "## @my_desktop\nnvidia\n";
        let result = parse_config_str(input);
        assert!(result.is_err());
    }
}
