use std::path::Path;
use std::process::Command;

pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_command_echo() {
        let output = run_command("echo hello", None).unwrap();
        assert_eq!(output.stdout.trim(), "hello");
        assert_eq!(output.exit_code, 0);
    }
}
