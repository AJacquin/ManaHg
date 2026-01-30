use std::path::{PathBuf};
use std::process::Command;
use anyhow::{Result, Context};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Clone, Debug)]
pub struct Repository {
    pub path: PathBuf,
    pub current_branch: String,
    pub revision: String,
    pub modified: bool,
    pub commit_type: String,
    pub last_status: String,
}

#[allow(dead_code)]
impl Repository {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            current_branch: "".to_string(),
            revision: "".to_string(),
            modified: false,
            commit_type: "".to_string(),
            last_status: "".to_string(),
        }
    }

    pub fn refresh(&mut self) {
        self.current_branch = self.get_current_branch().unwrap_or_else(|_| "ERROR".to_string());
        
        // Revision and Modified status
        if let Ok((rev, modded)) = self.get_repo_status() {
            self.revision = rev;
            self.modified = modded;
        } else {
            self.revision = "?".to_string();
            self.modified = false;
        }

        self.commit_type = self.get_commit_type().unwrap_or_else(|_| "Unknown".to_string());
    }

    fn run_hg(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("hg");
        command.args(args);
        command.current_dir(&self.path);
        
        // Hide console window on Windows when spawning hg commands
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command
            .output()
            .context("Failed to execute hg command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hg command failed: {}", stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn get_current_branch(&self) -> Result<String> {
        self.run_hg(&["branch"])
    }

    pub fn get_all_branches(&self) -> Result<Vec<String>> {
        let output = self.run_hg(&["branches"])?;
        let branches = output.lines()
            .map(|line| line.split_whitespace().next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(branches)
    }

    pub fn get_commit_type(&self) -> Result<String> {
        let output = self.run_hg(&["log", "-r", ".", "--template", "{phase}"])?;
        // Capitalize first letter
        let mut chars = output.chars();
        match chars.next() {
            None => Ok(String::new()),
            Some(f) => Ok(f.to_uppercase().collect::<String>() + chars.as_str()),
        }
    }

    pub fn pull_all_branches(&self) -> Result<String> {
        self.run_hg(&["pull"])
    }

    pub fn pull_current_branch(&self) -> Result<String> {
         if self.current_branch.starts_with("ERROR") {
             anyhow::bail!("Cannot pull: current branch unknown");
         }
         self.run_hg(&["pull", "-b", &self.current_branch])
    }

    pub fn update_to_latest(&self) -> Result<String> {
        self.run_hg(&["update"])
    }

    pub fn get_repo_status(&self) -> Result<(String, bool)> {
        let id_output = self.run_hg(&["id", "-n"])?;
        
        // Check for uncommitted changes
        let status_output = match self.run_hg(&["status", "-q"]) {
            Ok(s) => s,
            Err(_) => String::new(), // Treat error as no changes? Or propagate? Python logic: "ERROR" check
        };
        
        let has_changes = !status_output.is_empty();
        Ok((id_output, has_changes))
    }

    pub fn update_branch(&self, new_branch: &str) -> Result<String> {
        self.run_hg(&["update", new_branch])
    }

    pub fn revert_changes(&self) -> Result<String> {
        self.run_hg(&["revert", "--all"])
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        self.run_hg(&["commit", "-m", message])
    }

    pub fn update_to_last_public(&self) -> Result<String> {
        let branch = &self.current_branch;
        if branch.starts_with("ERROR") {
            anyhow::bail!("Unknown branch");
        }
        let rev_spec = format!("last(public() and branch(\"{}\"))", branch);
        self.run_hg(&["update", "-r", &rev_spec])
    }
}
