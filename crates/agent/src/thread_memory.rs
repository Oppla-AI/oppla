use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlez::connection::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

/// Represents a compressed memory of a conversation thread
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThreadMemory {
    pub id: String,
    pub thread_id: String,
    pub original_message_ids: Vec<String>,
    pub start_message_id: String,
    pub end_message_id: String,
    pub compressed_content: String,
    pub summary: String,
    pub compression_ratio: f32,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub tokens_saved: usize,
    pub user_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetadata {
    pub conversation_range: String,
    pub key_topics: Vec<String>,
    pub content_type: String,
    pub priority: u8,
}

impl ThreadMemory {
    pub fn new(
        thread_id: String,
        original_message_ids: Vec<String>,
        compressed_content: String,
        summary: String,
        original_tokens: usize,
        compressed_tokens: usize,
    ) -> Self {
        let now = Utc::now();
        let start_message_id = original_message_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let end_message_id = original_message_ids
            .last()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            id: Uuid::new_v4().to_string(),
            thread_id,
            original_message_ids,
            start_message_id: start_message_id.clone(),
            end_message_id: end_message_id.clone(),
            compressed_content,
            summary,
            compression_ratio: compressed_tokens as f32 / original_tokens as f32,
            original_tokens,
            compressed_tokens,
            tokens_saved: original_tokens - compressed_tokens,
            user_edited: false,
            created_at: now,
            updated_at: now,
            edited_at: None,
            is_active: true,
            metadata: MemoryMetadata {
                conversation_range: format!("Messages {}-{}", start_message_id, end_message_id),
                key_topics: Vec::new(),
                content_type: "general".to_string(),
                priority: 5,
            },
        }
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }

    pub fn update_content(&mut self, new_content: String, new_summary: String) {
        self.compressed_content = new_content;
        self.summary = new_summary;
        self.user_edited = true;
        self.edited_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn compression_percentage(&self) -> f32 {
        (1.0 - self.compression_ratio) * 100.0
    }
}

/// Manages thread memories with persistence and retrieval
#[derive(Debug, Clone)]
pub struct ThreadMemoryManager {
    memories: Vec<Arc<ThreadMemory>>,
    total_saved_tokens: usize,
}

impl ThreadMemoryManager {
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
            total_saved_tokens: 0,
        }
    }

    pub fn add_memory(&mut self, memory: Arc<ThreadMemory>) {
        self.total_saved_tokens += memory.tokens_saved;
        self.memories.push(memory);
    }

    pub fn get_memories(&self) -> &[Arc<ThreadMemory>] {
        &self.memories
    }

    pub fn get_active_memories(&self) -> Vec<Arc<ThreadMemory>> {
        self.memories
            .iter()
            .filter(|m| m.is_active)
            .cloned()
            .collect()
    }

    pub fn find_memory(&self, memory_id: &str) -> Option<Arc<ThreadMemory>> {
        self.memories.iter().find(|m| m.id == memory_id).cloned()
    }

    pub fn update_memory(
        &mut self,
        memory_id: &str,
        new_content: String,
        new_summary: String,
    ) -> Result<()> {
        if let Some(memory) = self.memories.iter_mut().find(|m| m.id == memory_id) {
            let mut updated_memory = (**memory).clone();
            updated_memory.update_content(new_content, new_summary);
            *memory = Arc::new(updated_memory);
        }
        Ok(())
    }

    pub fn deactivate_memory(&mut self, memory_id: &str) -> Result<()> {
        if let Some(memory) = self.memories.iter_mut().find(|m| m.id == memory_id) {
            let mut updated_memory = (**memory).clone();
            updated_memory.deactivate();
            let tokens_saved = updated_memory.tokens_saved;
            *memory = Arc::new(updated_memory);
            self.total_saved_tokens = self.total_saved_tokens.saturating_sub(tokens_saved);
        }
        Ok(())
    }

    pub fn deactivate_memories_before(&mut self, message_id: &str) -> Result<()> {
        for memory in &mut self.memories {
            if memory.end_message_id <= message_id.to_string() {
                let mut updated_memory = (**memory).clone();
                updated_memory.deactivate();
                let tokens_saved = updated_memory.tokens_saved;
                *memory = Arc::new(updated_memory);
                self.total_saved_tokens = self.total_saved_tokens.saturating_sub(tokens_saved);
            }
        }
        Ok(())
    }

    pub fn deactivate_all_memories(&mut self) {
        for memory in &mut self.memories {
            let mut updated_memory = (**memory).clone();
            updated_memory.deactivate();
            *memory = Arc::new(updated_memory);
        }
        self.total_saved_tokens = 0;
    }

    pub fn remove_memory(&mut self, memory_id: &str) -> Result<()> {
        if let Some(index) = self.memories.iter().position(|m| m.id == memory_id) {
            let removed = self.memories.remove(index);
            self.total_saved_tokens = self.total_saved_tokens.saturating_sub(removed.tokens_saved);
        }
        Ok(())
    }

    pub fn total_tokens_saved(&self) -> usize {
        self.total_saved_tokens
    }

    pub fn total_active_compressed_tokens(&self) -> usize {
        self.get_active_memories()
            .iter()
            .map(|m| m.compressed_tokens)
            .sum()
    }

    pub fn build_compressed_context(&self) -> String {
        let active_memories = self.get_active_memories();
        if active_memories.is_empty() {
            return String::new();
        }

        let mut context = String::from("=== Thread Memories ===\n");
        for memory in active_memories {
            context.push_str(&format!(
                "[Memory: {}]\n{}\n\n",
                memory.summary, memory.compressed_content
            ));
        }
        context.push_str("=== End of Memories ===\n");
        context
    }

    pub fn save_memory_to_db(
        &self,
        memory: &ThreadMemory,
        connection: &Arc<Mutex<Connection>>,
    ) -> Result<()> {
        let memory_json = serde_json::to_string(memory)?;

        let conn = connection.lock().unwrap();

        conn.exec_bound::<(String, String, String, f32, i64, String)>(indoc::indoc! {"
            INSERT OR REPLACE INTO thread_memories (
                id, thread_id, summary, compression_ratio, tokens_saved, data
            ) VALUES (?, ?, ?, ?, ?, ?)
        "})?((
            memory.id.clone(),
            memory.thread_id.clone(),
            memory.summary.clone(),
            memory.compression_ratio,
            memory.tokens_saved as i64,
            memory_json,
        ))?;

        Ok(())
    }

    pub fn load_memories_from_db(
        &mut self,
        thread_id: &str,
        connection: &Arc<Mutex<Connection>>,
    ) -> Result<()> {
        let conn = connection.lock().unwrap();

        type MemoryRow = (String, String, String, f32, i64, String);

        let mut select = conn.select_bound::<&str, MemoryRow>(indoc::indoc! {"
            SELECT id, thread_id, summary, compression_ratio, tokens_saved, data
            FROM thread_memories
            WHERE thread_id = ?
            ORDER BY id DESC
        "})?;

        self.memories.clear();
        self.total_saved_tokens = 0;

        let rows = select(thread_id)?;

        for (_id, _thread_id, _summary, _ratio, tokens_saved, data_json) in rows {
            let memory: ThreadMemory = serde_json::from_str(&data_json)?;

            self.total_saved_tokens += tokens_saved as usize;
            self.memories.push(Arc::new(memory));
        }

        Ok(())
    }

    pub fn delete_memory_from_db(
        &self,
        memory_id: &str,
        connection: &Arc<Mutex<Connection>>,
    ) -> Result<()> {
        let conn = connection.lock().unwrap();

        let sql = "DELETE FROM thread_memories WHERE id = ?";
        let mut delete = conn.exec_bound::<&str>(sql)?;
        delete(memory_id)?;

        Ok(())
    }
}

pub fn mock_compress_messages(
    messages: &[String],
    target_ratio: f32,
) -> (String, String, usize, usize) {
    // Join messages with space separator to avoid encoding issues
    let combined = messages.join(" ");
    let original_tokens = combined.len() / 4;
    let target_tokens = (original_tokens as f32 * target_ratio) as usize;

    let compressed = if combined.len() > 200 {
        format!("{}...", &combined[..200])
    } else {
        combined.clone()
    };

    let summary = format!("Compressed {} messages", messages.len());

    (compressed, summary, original_tokens, target_tokens)
}
