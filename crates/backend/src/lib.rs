use std::path::Path;
use std::process::Command;

pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
    pub is_symlink: bool,
}

fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("powershell.exe");
        cmd.arg("-NoProfile").arg("-Command").arg(command);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

pub fn run_command(command: &str, cwd: Option<&Path>) -> Result<CommandOutput, std::io::Error> {
    let mut cmd = shell_command(command);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;
    Ok(CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub fn list_directory(path: &Path) -> Result<Vec<DirectoryEntry>, std::io::Error> {
    #[cfg(windows)]
    {
        let command =
            "Get-ChildItem -Force | ForEach-Object { $_.Mode.Substring(0,1) + ' ' + $_.Name }";
        let output = run_command(command, Some(path))?;
        Ok(parse_powershell_listing(&output.stdout))
    }
    #[cfg(not(windows))]
    {
        let output = run_command("ls -la", Some(path))?;
        Ok(parse_ls_output(&output.stdout))
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse_powershell_listing(stdout: &str) -> Vec<DirectoryEntry> {
    stdout.lines().filter_map(parse_powershell_line).collect()
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse_powershell_line(line: &str) -> Option<DirectoryEntry> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }
    let mut chars = line.chars();
    let kind = chars.next()?;
    let name = chars.as_str().trim_start_matches(' ').to_string();
    if name.is_empty() {
        return None;
    }
    Some(DirectoryEntry {
        name,
        is_directory: kind == 'd',
        is_symlink: kind == 'l',
    })
}

fn parse_ls_output(stdout: &str) -> Vec<DirectoryEntry> {
    stdout.lines().filter_map(parse_ls_line).collect()
}

fn parse_ls_line(line: &str) -> Option<DirectoryEntry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("total ") {
        return None;
    }
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let perms = tokens[0];

    let name = if tokens.len() >= 9 {
        tokens[8..].join(" ").to_string()
    } else {
        tokens.last().unwrap().to_string()
    };
    let name = name.split(" -> ").next().unwrap_or(&name).to_string();

    if name == "." || name == ".." {
        return None;
    }

    Some(DirectoryEntry {
        name,
        is_directory: perms.starts_with('d'),
        is_symlink: perms.starts_with('l'),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ls_line_directory() {
        let entry = parse_ls_line("drwxr-xr-x  5 user  staff  160 Jan  1 12:00 src").unwrap();
        assert!(entry.is_directory);
        assert!(!entry.is_symlink);
        assert_eq!(entry.name, "src");
    }

    #[test]
    fn parse_ls_line_file() {
        let entry = parse_ls_line("-rw-r--r--  1 user  staff  120 Jan  1 12:00 main.rs").unwrap();
        assert!(!entry.is_directory);
        assert_eq!(entry.name, "main.rs");
    }

    #[test]
    fn parse_ls_line_name_with_spaces() {
        let entry =
            parse_ls_line("-rw-r--r--  1 user  staff  120 Jan  1 12:00 my file.txt").unwrap();
        assert_eq!(entry.name, "my file.txt");
    }

    #[test]
    fn parse_ls_line_symlink() {
        let entry =
            parse_ls_line("lrwxr-xr-x  1 user  staff    0 Jan  1 12:00 link -> target").unwrap();
        assert!(entry.is_symlink);
        assert_eq!(entry.name, "link");
    }

    #[test]
    fn parse_ls_line_ignores_total_and_empties() {
        assert!(parse_ls_line("").is_none());
        assert!(parse_ls_line("total 8").is_none());
    }

    #[test]
    fn parse_ls_line_skips_dot_entries() {
        assert!(parse_ls_line("drwxr-xr-x 5 user staff 160 Jan 1 12:00 .").is_none());
        assert!(parse_ls_line("drwxr-xr-x 5 user staff 160 Jan 1 12:00 ..").is_none());
    }

    #[test]
    fn parses_powershell_listing() {
        let out = "d .git\n- Cargo.toml\nl link\n\n";
        let entries = parse_powershell_listing(out);
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, ".git");
        assert!(!entries[1].is_directory);
        assert_eq!(entries[1].name, "Cargo.toml");
        assert!(entries[2].is_symlink);
        assert_eq!(entries[2].name, "link");
    }

    #[test]
    fn parses_powershell_listing_name_with_spaces() {
        let out = "- my file.txt\n";
        let entries = parse_powershell_listing(out);
        assert_eq!(entries[0].name, "my file.txt");
    }

    #[test]
    fn run_command_echo() {
        let output = run_command("echo hello", None).unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn list_directory_contains_this_crate() {
        let entries = list_directory(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(entries.iter().any(|e| e.name == "Cargo.toml"));
    }
}
