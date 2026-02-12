use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Config {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub header: Header,
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    All,
    Hostname(String),
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub source: PackageSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PackageSource {
    Official,
    Aur,
}

impl std::fmt::Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Header::All => write!(f, "## *"),
            Header::Hostname(h) => write!(f, "## @{h}"),
        }
    }
}

/// Collect packages from config for a given hostname.
/// Returns (official_packages, aur_packages) with duplicates removed (first occurrence kept).
pub fn collect_packages(config: &Config, hostname: &str) -> (Vec<String>, Vec<String>) {
    let mut official = Vec::new();
    let mut aur = Vec::new();
    let mut seen = HashSet::new();

    for section in &config.sections {
        let should_include = match &section.header {
            Header::All => true,
            Header::Hostname(h) => h == hostname,
        };

        if should_include {
            for package in &section.packages {
                if seen.insert(package.name.clone()) {
                    match package.source {
                        PackageSource::Official => official.push(package.name.clone()),
                        PackageSource::Aur => aur.push(package.name.clone()),
                    }
                }
            }
        }
    }

    (official, aur)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pkg(name: &str, source: PackageSource) -> Package {
        Package {
            name: name.to_string(),
            source,
        }
    }

    #[test]
    fn test_collect_wildcard_only() {
        let config = Config {
            sections: vec![Section {
                header: Header::All,
                packages: vec![
                    make_pkg("base", PackageSource::Official),
                    make_pkg("git", PackageSource::Official),
                    make_pkg("yay", PackageSource::Aur),
                ],
            }],
        };
        let (official, aur) = collect_packages(&config, "myhost");
        assert_eq!(official, vec!["base", "git"]);
        assert_eq!(aur, vec!["yay"]);
    }

    #[test]
    fn test_collect_hostname_filtering() {
        let config = Config {
            sections: vec![
                Section {
                    header: Header::All,
                    packages: vec![make_pkg("base", PackageSource::Official)],
                },
                Section {
                    header: Header::Hostname("desktop".to_string()),
                    packages: vec![make_pkg("nvidia", PackageSource::Official)],
                },
                Section {
                    header: Header::Hostname("laptop".to_string()),
                    packages: vec![make_pkg("tlp", PackageSource::Official)],
                },
            ],
        };
        let (official, aur) = collect_packages(&config, "desktop");
        assert_eq!(official, vec!["base", "nvidia"]);
        assert!(aur.is_empty());
    }

    #[test]
    fn test_collect_dedup_keeps_first() {
        let config = Config {
            sections: vec![
                Section {
                    header: Header::All,
                    packages: vec![make_pkg("firefox", PackageSource::Official)],
                },
                Section {
                    header: Header::Hostname("myhost".to_string()),
                    packages: vec![make_pkg("firefox", PackageSource::Official)],
                },
            ],
        };
        let (official, _) = collect_packages(&config, "myhost");
        assert_eq!(official, vec!["firefox"]);
    }

    #[test]
    fn test_collect_merges_same_hostname_sections() {
        let config = Config {
            sections: vec![
                Section {
                    header: Header::Hostname("desktop".to_string()),
                    packages: vec![make_pkg("nvidia", PackageSource::Official)],
                },
                Section {
                    header: Header::All,
                    packages: vec![make_pkg("firefox", PackageSource::Official)],
                },
                Section {
                    header: Header::Hostname("desktop".to_string()),
                    packages: vec![make_pkg("steam", PackageSource::Official)],
                },
            ],
        };
        let (official, _) = collect_packages(&config, "desktop");
        assert_eq!(official, vec!["nvidia", "firefox", "steam"]);
    }

    #[test]
    fn test_collect_no_matching_hostname() {
        let config = Config {
            sections: vec![Section {
                header: Header::Hostname("other".to_string()),
                packages: vec![make_pkg("nvidia", PackageSource::Official)],
            }],
        };
        let (official, aur) = collect_packages(&config, "myhost");
        assert!(official.is_empty());
        assert!(aur.is_empty());
    }
}
