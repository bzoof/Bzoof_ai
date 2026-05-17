use anyhow::{anyhow, Result};

const ALLOWED_COMMANDS: &[&str] = &[
    "ls", "echo", "cat", "head", "tail", "grep", "find",
    "pwd", "whoami", "date", "uptime", "df", "du", "free",
    "ps", "wc", "sort", "uniq", "cut", "tr", "diff",
    "file", "stat", "md5sum", "sha256sum", "git", "cargo",
];

pub struct ShellRunner;

impl ShellRunner {
    pub fn sanitize_command(raw: &str) -> Result<(String, Vec<String>)> {
        let trimmed = raw.trim();

        // Check empty command
        if trimmed.is_empty() {
            return Err(anyhow!("empty command"));
        }

        // Check max length
        if trimmed.len() > 512 {
            return Err(anyhow!("command too long (max 512 chars)"));
        }

        // Reject null bytes and some control characters
        if trimmed.contains('\0') || trimmed.bytes().any(|b| b < 32 && b != 9 && b != 10) {
            return Err(anyhow!("invalid control characters in command"));
        }

        // Tokenize using shell_words
        let tokens = shell_words::split(trimmed)
            .map_err(|e| anyhow!("failed to parse command: {}", e))?;

        if tokens.is_empty() {
            return Err(anyhow!("empty command after tokenization"));
        }

        // Check arg count
        if tokens.len() > 20 {
            return Err(anyhow!("too many arguments (max 20)"));
        }

        let prog = &tokens[0];

        // Check if command is allowed
        if !ALLOWED_COMMANDS.contains(&prog.as_str()) {
            return Err(anyhow!("command '{}' is not allowed", prog));
        }

        // Reject shell metacharacters in arguments (unless inside quotes, which shell_words handles)
        for arg in &tokens[1..] {
            if arg.contains(';')
                || arg.contains('&')
                || arg.contains('|')
                || arg.contains('>')
                || arg.contains('<')
                || arg.contains('`')
                || arg.contains('$')
                || arg.contains('(')
                || arg.contains(')')
                || arg.contains('{')
                || arg.contains('}')
                || arg.contains('[')
                || arg.contains(']')
                || arg.contains('#')
                || arg.contains('!')
            {
                return Err(anyhow!("shell metacharacters not allowed in arguments"));
            }

            // Reject path traversal attempts
            if arg.contains("..") {
                return Err(anyhow!("path traversal (..) not allowed"));
            }
        }

        let args = tokens[1..].to_vec();

        tracing::warn!(
            prog = %prog,
            args_count = args.len(),
            "shell_runner: command allowed"
        );

        Ok((prog.to_string(), args))
    }

    pub async fn run_command(
        command: &str,
        tx: tokio::sync::mpsc::Sender<super::chat_ui::UiEvent>,
    ) -> Result<()> {
        let (prog, args) = Self::sanitize_command(command)?;

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new(&prog)
                .args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .current_dir("/tmp")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .output(),
        )
        .await??;

        let stdout_str = String::from_utf8_lossy(&output.stdout[..output.stdout.len().min(4096)]);
        let stderr_str = String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(512)]);

        if output.status.success() {
            tx.send(super::chat_ui::UiEvent::ShellOutput(stdout_str.to_string()))
                .await?;
        } else {
            tx.send(super::chat_ui::UiEvent::ShellError(format!(
                "Exit {}: {}",
                output.status, stderr_str
            )))
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_command() {
        let result = ShellRunner::sanitize_command("ls -la /tmp");
        assert!(result.is_ok());
        let (prog, args) = result.unwrap();
        assert_eq!(prog, "ls");
        assert_eq!(args, vec!["-la", "/tmp"]);
    }

    #[test]
    fn test_blocked_command() {
        let result = ShellRunner::sanitize_command("rm -rf /");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not allowed"));
    }

    #[test]
    fn test_empty_command() {
        let result = ShellRunner::sanitize_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_metacharacter_rejection() {
        let result = ShellRunner::sanitize_command("ls; rm foo");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_rejection() {
        let result = ShellRunner::sanitize_command("cat ../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_command_too_long() {
        let long_cmd = format!("ls {}", "x".repeat(600));
        let result = ShellRunner::sanitize_command(&long_cmd);
        assert!(result.is_err());
    }
}
