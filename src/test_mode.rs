use std::collections::HashSet;
use std::fmt;

use crate::pane_module::ModuleId;

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMode {
    pub active_modules: HashSet<ModuleId>,
}

impl Default for TestMode {
    fn default() -> Self {
        Self::auto_queue()
    }
}

impl TestMode {
    #[allow(dead_code)]
    pub fn new(modules: impl IntoIterator<Item = ModuleId>) -> Self {
        Self {
            active_modules: modules.into_iter().collect(),
        }
    }

    pub fn single(module: ModuleId) -> Self {
        let mut set = HashSet::new();
        set.insert(module);
        Self { active_modules: set }
    }

    pub fn auto_queue() -> Self {
        Self::single(ModuleId::AutoQueue)
    }

    pub fn is_active(&self, module: ModuleId) -> bool {
        self.active_modules.contains(&module)
    }

    pub fn single_active_module(&self) -> Option<ModuleId> {
        if self.active_modules.len() == 1 {
            self.active_modules.iter().next().copied()
        } else {
            None
        }
    }

    pub fn from_names(names: &[&str]) -> Self {
        let mut modules = HashSet::new();
        for &name in names {
            if name.eq_ignore_ascii_case("all") {
                modules.insert(ModuleId::AutoQueue);
                modules.insert(ModuleId::Text);
                modules.insert(ModuleId::Upgrade);
                modules.insert(ModuleId::Graph);
                modules.insert(ModuleId::MoneyBar);
            } else if let Some(m) = ModuleId::parse(name) {
                modules.insert(m);
            }
        }
        if modules.is_empty() {
            modules.insert(ModuleId::AutoQueue);
        }
        Self {
            active_modules: modules,
        }
    }

    pub fn parse_csv(csv: &str) -> Self {
        let parts: Vec<&str> = csv.split(',').collect();
        Self::from_names(&parts)
    }
}

impl fmt::Display for TestMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list: Vec<&'static str> = self.active_modules.iter().map(|m| m.name()).collect();
        list.sort();
        write!(f, "[{}]", list.join(", "))
    }
}

pub fn parse_save_path_from_args(args: &[String]) -> Option<std::path::PathBuf> {
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--save" || arg == "-s" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                return Some(std::path::PathBuf::from(&args[i + 1]));
            }
        } else if let Some(val) = arg.strip_prefix("--save=") {
            return Some(std::path::PathBuf::from(val));
        }
        i += 1;
    }
    None
}

pub fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

pub fn parse_test_mode_from_args(args: &[String]) -> Option<TestMode> {
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--test"
            || arg == "-t"
            || arg == "--test-mode"
            || arg == "--test-module"
            || arg == "--module"
        {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                return Some(TestMode::parse_csv(&args[i + 1]));
            } else {
                return Some(TestMode::auto_queue());
            }
        } else if let Some(val) = arg.strip_prefix("--test=") {
            return Some(TestMode::parse_csv(val));
        } else if let Some(val) = arg.strip_prefix("--test-mode=") {
            return Some(TestMode::parse_csv(val));
        } else if let Some(val) = arg.strip_prefix("--test-module=") {
            return Some(TestMode::parse_csv(val));
        } else if let Some(val) = arg.strip_prefix("--module=") {
            return Some(TestMode::parse_csv(val));
        }
        i += 1;
    }

    if let Ok(env_val) = std::env::var("TEST_MODE").or_else(|_| std::env::var("TEST_MODULE")) {
        if !env_val.trim().is_empty() {
            return Some(TestMode::parse_csv(&env_val));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_module_parse() {
        assert_eq!(ModuleId::parse("auto-queue"), Some(ModuleId::AutoQueue));
        assert_eq!(ModuleId::parse("auto_queue"), Some(ModuleId::AutoQueue));
        assert_eq!(ModuleId::parse("keys"), Some(ModuleId::AutoQueue));
        assert_eq!(ModuleId::parse("text"), Some(ModuleId::Text));
        assert_eq!(ModuleId::parse("upgrade"), Some(ModuleId::Upgrade));
        assert_eq!(ModuleId::parse("graph"), Some(ModuleId::Graph));
        assert_eq!(ModuleId::parse("money-bar"), Some(ModuleId::MoneyBar));
        assert_eq!(ModuleId::parse("unknown"), None);
    }

    #[test]
    fn test_test_mode_default_is_auto_queue() {
        let tm = TestMode::default();
        assert!(!tm.is_active(ModuleId::AutoQueue));
        assert!(!tm.is_active(ModuleId::Text));
        assert!(tm.single_active_module() != Some(ModuleId::AutoQueue));
    }

    #[test]
    fn test_parse_csv() {
        let tm = TestMode::parse_csv("auto-queue,text");
        assert!(tm.is_active(ModuleId::AutoQueue));
        assert!(tm.is_active(ModuleId::Text));
        assert!(!tm.is_active(ModuleId::Upgrade));
        assert!(tm.single_active_module() != Some(ModuleId::AutoQueue));
    }

    #[test]
    fn test_parse_args_flags() {
        let args = vec!["idle_game".to_string(), "--test".to_string()];
        let tm = parse_test_mode_from_args(&args).unwrap();
        assert!(tm.single_active_module() == Some(ModuleId::AutoQueue));

        let args = vec![
            "idle_game".to_string(),
            "--test".to_string(),
            "auto-queue".to_string(),
        ];
        let tm = parse_test_mode_from_args(&args).unwrap();
        assert!(tm.single_active_module() == Some(ModuleId::AutoQueue));

        let args = vec![
            "idle_game".to_string(),
            "--test-mode=text".to_string(),
        ];
        let tm = parse_test_mode_from_args(&args).unwrap();
        assert!(tm.single_active_module() == Some(ModuleId::Text));

        let args = vec!["idle_game".to_string()];
        assert_eq!(parse_test_mode_from_args(&args), None);
    }
}
