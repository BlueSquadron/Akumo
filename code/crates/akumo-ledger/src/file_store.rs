//! A JSON-Lines, file-per-engagement implementation of the [`EventStore`] port.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::event::EventEnvelope;
use akumo_domain::ids::{EngagementId, EventHash, Seq};
use akumo_domain::ports::EventStore;

/// The head of one engagement's chain: the last sequence number and hash appended.
#[derive(Clone, Debug)]
struct ChainHead {
    seq: Seq,
    hash: EventHash,
}

/// A durable, append-only event store keeping one JSON-Lines file per engagement.
pub struct FileEventStore {
    root: PathBuf,
    /// Cached chain heads, rebuilt from disk on [`FileEventStore::open`].
    heads: Mutex<HashMap<EngagementId, ChainHead>>,
}

impl FileEventStore {
    /// Open (creating if necessary) a store rooted at `root`, rebuilding chain heads from any
    /// existing engagement files so appends resume correctly after a restart (NFR-REL2).
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let mut heads: HashMap<EngagementId, ChainHead> = HashMap::new();
        for entry in fs::read_dir(&root)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let events = read_file(&path)?;
            if let Some(last) = events.last() {
                heads.insert(
                    EngagementId::new(stem),
                    ChainHead { seq: last.seq, hash: last.hash.clone() },
                );
            }
        }

        Ok(Self { root, heads: Mutex::new(heads) })
    }

    fn file_path(&self, engagement: &EngagementId) -> PathBuf {
        self.root.join(format!("{}.jsonl", sanitize(engagement.as_str())))
    }

    fn append_sync(&self, event: EventEnvelope) -> Result<()> {
        let mut heads = self.lock()?;
        let head = heads.get(&event.engagement_id).cloned();

        // Enforce chain linkage: prev_hash must equal the current head (or None for genesis).
        let expected_prev = head.as_ref().map(|h| h.hash.clone());
        if event.prev_hash != expected_prev {
            return Err(AkumoError::Integrity(format!(
                "append rejected for {}: prev_hash does not match chain head",
                event.engagement_id
            )));
        }

        // Enforce ordering: seq must be the next in sequence.
        let expected_seq = head.as_ref().map(|h| h.seq.next()).unwrap_or(Seq::ZERO);
        if event.seq != expected_seq {
            return Err(AkumoError::Integrity(format!(
                "append rejected for {}: expected seq {}, got {}",
                event.engagement_id, expected_seq.0, event.seq.0
            )));
        }

        // Enforce integrity: the event's stored hash must match its content.
        event.verify_hash()?;

        // Durable append.
        let path = self.file_path(&event.engagement_id);
        let line = serde_json::to_string(&event)?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;

        heads.insert(
            event.engagement_id.clone(),
            ChainHead { seq: event.seq, hash: event.hash.clone() },
        );
        Ok(())
    }

    fn read_stream_sync(&self, engagement: &EngagementId) -> Result<Vec<EventEnvelope>> {
        let path = self.file_path(engagement);
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_file(&path)
    }

    fn last_hash_sync(&self, engagement: &EngagementId) -> Result<Option<EventHash>> {
        let heads = self.lock()?;
        Ok(heads.get(engagement).map(|h| h.hash.clone()))
    }

    /// Verify an engagement's full chain: every event hash checks out and each links to its
    /// predecessor with a contiguous sequence. Used by audit/tests (NFR-OBS3).
    pub fn verify_chain(&self, engagement: &EngagementId) -> Result<()> {
        let events = self.read_stream_sync(engagement)?;
        let mut prev: Option<&EventHash> = None;
        let mut expected_seq = Seq::ZERO;
        for event in &events {
            event.verify_hash()?;
            if event.prev_hash.as_ref() != prev {
                return Err(AkumoError::Integrity(format!(
                    "chain break at seq {} in {engagement}",
                    event.seq.0
                )));
            }
            if event.seq != expected_seq {
                return Err(AkumoError::Integrity(format!(
                    "sequence gap in {engagement}: expected {}, got {}",
                    expected_seq.0, event.seq.0
                )));
            }
            prev = Some(&event.hash);
            expected_seq = expected_seq.next();
        }
        Ok(())
    }

    /// List the engagement ids that have a ledger file under the store root.
    pub fn list_engagements(&self) -> Result<Vec<EngagementId>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    out.push(EngagementId::new(stem));
                }
            }
        }
        out.sort();
        Ok(out)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<EngagementId, ChainHead>>> {
        self.heads
            .lock()
            .map_err(|_| AkumoError::Persistence("event store lock poisoned".to_string()))
    }
}

#[async_trait]
impl EventStore for FileEventStore {
    async fn append(&self, event: EventEnvelope) -> Result<()> {
        self.append_sync(event)
    }

    async fn read_stream(&self, engagement: &EngagementId) -> Result<Vec<EventEnvelope>> {
        self.read_stream_sync(engagement)
    }

    async fn last_hash(&self, engagement: &EngagementId) -> Result<Option<EventHash>> {
        self.last_hash_sync(engagement)
    }

    async fn head(&self, engagement: &EngagementId) -> Result<Option<(Seq, EventHash)>> {
        let heads = self.lock()?;
        Ok(heads.get(engagement).map(|h| (h.seq, h.hash.clone())))
    }
}

/// Read and parse every non-empty line of a JSON-Lines event file.
fn read_file(path: &Path) -> Result<Vec<EventEnvelope>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str::<EventEnvelope>(&line)?);
    }
    Ok(out)
}

/// Reduce an engagement id to a filesystem-safe file stem. v1 ids are id-like; this is defensive.
fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::ids::{Actor, Timestamp};

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("akumo-ledger-test-{tag}-{nanos}"));
        p
    }

    fn event(eng: &EngagementId, prev: Option<EventHash>, seq: u64) -> EventEnvelope {
        EventEnvelope::seal(
            format!("evt-{seq}"),
            eng.clone(),
            Seq(seq),
            Timestamp::from_millis(seq),
            Actor::new("tester"),
            prev,
            "TestEvent",
            serde_json::json!({ "n": seq }),
        )
        .unwrap()
    }

    #[test]
    fn append_read_and_chain_verify() {
        let root = temp_root("append");
        let store = FileEventStore::open(&root).unwrap();
        let eng = EngagementId::new("eng-append");

        let e0 = event(&eng, None, 0);
        let e1 = event(&eng, Some(e0.hash.clone()), 1);
        store.append_sync(e0.clone()).unwrap();
        store.append_sync(e1.clone()).unwrap();

        let stream = store.read_stream_sync(&eng).unwrap();
        assert_eq!(stream.len(), 2);
        assert_eq!(store.last_hash_sync(&eng).unwrap(), Some(e1.hash.clone()));
        store.verify_chain(&eng).unwrap();

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn append_rejects_wrong_prev_or_seq() {
        let root = temp_root("reject");
        let store = FileEventStore::open(&root).unwrap();
        let eng = EngagementId::new("eng-reject");

        let e0 = event(&eng, None, 0);
        store.append_sync(e0.clone()).unwrap();

        // Wrong prev_hash (None instead of e0.hash).
        let bad_prev = event(&eng, None, 1);
        assert!(store.append_sync(bad_prev).is_err());

        // Wrong seq (skips 1).
        let bad_seq = event(&eng, Some(e0.hash.clone()), 2);
        assert!(store.append_sync(bad_seq).is_err());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reopen_rebuilds_head_and_isolates_engagements() {
        let root = temp_root("reopen");
        let eng_a = EngagementId::new("eng-a");
        let eng_b = EngagementId::new("eng-b");

        {
            let store = FileEventStore::open(&root).unwrap();
            let a0 = event(&eng_a, None, 0);
            store.append_sync(a0.clone()).unwrap();
            let a1 = event(&eng_a, Some(a0.hash.clone()), 1);
            store.append_sync(a1).unwrap();
            store.append_sync(event(&eng_b, None, 0)).unwrap();
        }

        // Reopen: heads must be rebuilt from disk (NFR-REL2), engagements isolated (DSR-2).
        let store = FileEventStore::open(&root).unwrap();
        assert_eq!(store.read_stream_sync(&eng_a).unwrap().len(), 2);
        assert_eq!(store.read_stream_sync(&eng_b).unwrap().len(), 1);
        // Appending the next event for A must succeed against the rebuilt head.
        let head = store.last_hash_sync(&eng_a).unwrap().unwrap();
        store.append_sync(event(&eng_a, Some(head), 2)).unwrap();
        assert_eq!(store.read_stream_sync(&eng_a).unwrap().len(), 3);

        fs::remove_dir_all(&root).ok();
    }
}
