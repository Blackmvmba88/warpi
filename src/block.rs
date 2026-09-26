use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockStatus {
    Pending,
    Running,
    Success,
    Error(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBlock {
    pub id: usize,
    pub timestamp: String,
    pub command: String,
    pub output: String,
    pub status: BlockStatus,
    pub ai_suggestion: Option<String>,
    pub suggested_fix: Option<String>,
}

impl CommandBlock {
    pub fn new(id: usize, command: String) -> Self {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        Self {
            id,
            timestamp,
            command,
            output: String::new(),
            status: BlockStatus::Pending,
            ai_suggestion: None,
            suggested_fix: None,
        }
    }

    pub fn append_output(&mut self, text: &str) {
        self.output.push_str(text);
    }

    pub fn mark_finished(&mut self, exit_code: i32) {
        if exit_code == 0 {
            self.status = BlockStatus::Success;
        } else {
            self.status = BlockStatus::Error(exit_code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_block_creation() {
        let block = CommandBlock::new(1, "echo Hello".to_string());
        assert_eq!(block.id, 1);
        assert_eq!(block.command, "echo Hello");
        assert_eq!(block.status, BlockStatus::Pending);
        assert!(block.output.is_empty());
    }

    #[test]
    fn test_block_output_and_status() {
        let mut block = CommandBlock::new(2, "ls -la".to_string());
        block.append_output("file1.txt\nfile2.txt");
        assert_eq!(block.output, "file1.txt\nfile2.txt");

        block.mark_finished(0);
        assert_eq!(block.status, BlockStatus::Success);

        block.mark_finished(1);
        assert_eq!(block.status, BlockStatus::Error(1));
    }
}
