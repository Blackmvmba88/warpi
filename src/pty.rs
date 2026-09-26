use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Read;

pub struct ShellRunner;

impl ShellRunner {
    pub fn run_command(cmd_str: &str) -> Result<(String, i32)> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.arg("-c");
        cmd.arg(cmd_str);

        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave); // Drop slave so reader gets EOF when child terminates

        let mut reader = pair.master.try_clone_reader()?;
        let mut output = String::new();
        let mut buf = [0u8; 1024];

        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let s = String::from_utf8_lossy(&buf[..n]);
            output.push_str(&s);
        }

        let status = child.wait()?;
        let exit_code = if status.success() { 0 } else { 1 };

        // Clean up VT100 / ANSI escape sequences if needed
        let cleaned_bytes = strip_ansi_escapes::strip(&output);
        let cleaned_output = String::from_utf8_lossy(&cleaned_bytes).to_string();

        Ok((cleaned_output, exit_code))
    }
}

mod strip_ansi_escapes {
    pub fn strip(input: &str) -> Vec<u8> {
        let mut result = Vec::new();
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                i += 1;
            } else {
                result.push(bytes[i]);
                i += 1;
            }
        }
        result
    }
}
