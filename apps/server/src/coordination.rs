use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use redis::AsyncCommands;
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::identity::SessionTokenHash;

#[derive(Clone, Debug)]
pub struct PlayerLease {
    pub owner: Uuid,
    pub fence: i64,
}

#[derive(Debug, Error)]
pub enum CoordinationError {
    #[error("coordination backend failed")]
    Redis(#[from] redis::RedisError),
}

#[async_trait]
pub trait SessionCoordinator: Send + Sync {
    async fn cache_session(
        &self,
        token_hash: SessionTokenHash,
        player: Uuid,
        ttl: Duration,
    ) -> Result<(), CoordinationError>;
    async fn resolve(
        &self,
        token_hash: SessionTokenHash,
        ttl: Duration,
    ) -> Result<Option<Uuid>, CoordinationError>;
    async fn acquire_lease(
        &self,
        player: Uuid,
        owner: Uuid,
        ttl: Duration,
    ) -> Result<Option<PlayerLease>, CoordinationError>;
    async fn renew_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
        ttl: Duration,
    ) -> Result<bool, CoordinationError>;
    async fn release_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
    ) -> Result<(), CoordinationError>;
    async fn allow_command(
        &self,
        token_hash: SessionTokenHash,
        limit: u32,
        window: Duration,
    ) -> Result<bool, CoordinationError>;
    async fn is_ready(&self) -> bool;
}

pub type SharedSessionCoordinator = Arc<dyn SessionCoordinator>;

#[derive(Default)]
pub struct InMemorySessionCoordinator {
    inner: Mutex<MemoryState>,
}

#[derive(Default)]
struct MemoryState {
    sessions: HashMap<SessionTokenHash, Uuid>,
    leases: HashMap<Uuid, PlayerLease>,
    fences: HashMap<Uuid, i64>,
    rates: HashMap<SessionTokenHash, (tokio::time::Instant, u32)>,
}

#[async_trait]
impl SessionCoordinator for InMemorySessionCoordinator {
    async fn cache_session(
        &self,
        token_hash: SessionTokenHash,
        player: Uuid,
        _ttl: Duration,
    ) -> Result<(), CoordinationError> {
        self.inner.lock().await.sessions.insert(token_hash, player);
        Ok(())
    }

    async fn resolve(
        &self,
        token_hash: SessionTokenHash,
        _ttl: Duration,
    ) -> Result<Option<Uuid>, CoordinationError> {
        Ok(self.inner.lock().await.sessions.get(&token_hash).copied())
    }

    async fn acquire_lease(
        &self,
        player: Uuid,
        owner: Uuid,
        _ttl: Duration,
    ) -> Result<Option<PlayerLease>, CoordinationError> {
        let mut state = self.inner.lock().await;
        if state.leases.contains_key(&player) {
            return Ok(None);
        }
        let fence = state.fences.entry(player).or_default();
        *fence += 1;
        let lease = PlayerLease {
            owner,
            fence: *fence,
        };
        state.leases.insert(player, lease.clone());
        Ok(Some(lease))
    }

    async fn renew_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
        _ttl: Duration,
    ) -> Result<bool, CoordinationError> {
        let state = self.inner.lock().await;
        Ok(state
            .leases
            .get(&player)
            .is_some_and(|active| active.owner == lease.owner && active.fence == lease.fence))
    }

    async fn release_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
    ) -> Result<(), CoordinationError> {
        let mut state = self.inner.lock().await;
        if state
            .leases
            .get(&player)
            .is_some_and(|active| active.owner == lease.owner && active.fence == lease.fence)
        {
            state.leases.remove(&player);
        }
        Ok(())
    }

    async fn allow_command(
        &self,
        token_hash: SessionTokenHash,
        limit: u32,
        window: Duration,
    ) -> Result<bool, CoordinationError> {
        let mut state = self.inner.lock().await;
        let now = tokio::time::Instant::now();
        let entry = state.rates.entry(token_hash).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        Ok(entry.1 <= limit)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}

pub struct RedisSessionCoordinator {
    client: redis::Client,
}

impl RedisSessionCoordinator {
    pub fn new(url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            client: redis::Client::open(url)?,
        })
    }

    async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, redis::RedisError> {
        self.client.get_multiplexed_async_connection().await
    }
}

fn session_key(token_hash: SessionTokenHash) -> String {
    format!("eh:session:{}", token_hash.cache_key_suffix())
}
fn lease_key(player: Uuid) -> String {
    format!("eh:player:{player}:lease")
}
fn fence_key(player: Uuid) -> String {
    format!("eh:player:{player}:fence")
}
fn lease_value(lease: &PlayerLease) -> String {
    format!("{}:{}", lease.owner, lease.fence)
}

#[async_trait]
impl SessionCoordinator for RedisSessionCoordinator {
    async fn cache_session(
        &self,
        token_hash: SessionTokenHash,
        player: Uuid,
        ttl: Duration,
    ) -> Result<(), CoordinationError> {
        let mut connection = self.connection().await?;
        let _: () = connection
            .set_ex(session_key(token_hash), player.to_string(), ttl.as_secs())
            .await?;
        Ok(())
    }

    async fn resolve(
        &self,
        token_hash: SessionTokenHash,
        ttl: Duration,
    ) -> Result<Option<Uuid>, CoordinationError> {
        let mut connection = self.connection().await?;
        let key = session_key(token_hash);
        let value: Option<String> = connection.get(&key).await?;
        let Some(value) = value else { return Ok(None) };
        let _: bool = connection.expire(&key, ttl.as_secs() as i64).await?;
        Ok(Uuid::parse_str(&value).ok())
    }

    async fn acquire_lease(
        &self,
        player: Uuid,
        owner: Uuid,
        ttl: Duration,
    ) -> Result<Option<PlayerLease>, CoordinationError> {
        let script = redis::Script::new(
            r#"
            if redis.call('EXISTS', KEYS[1]) == 1 then return 0 end
            local fence = redis.call('INCR', KEYS[2])
            redis.call('PSETEX', KEYS[1], ARGV[2], ARGV[1] .. ':' .. fence)
            return fence
        "#,
        );
        let mut connection = self.connection().await?;
        let fence: i64 = script
            .key(lease_key(player))
            .key(fence_key(player))
            .arg(owner.to_string())
            .arg(ttl.as_millis() as u64)
            .invoke_async(&mut connection)
            .await?;
        Ok((fence > 0).then_some(PlayerLease { owner, fence }))
    }

    async fn renew_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
        ttl: Duration,
    ) -> Result<bool, CoordinationError> {
        let script = redis::Script::new("if redis.call('GET', KEYS[1]) == ARGV[1] then return redis.call('PEXPIRE', KEYS[1], ARGV[2]) else return 0 end");
        let mut connection = self.connection().await?;
        let renewed: i64 = script
            .key(lease_key(player))
            .arg(lease_value(lease))
            .arg(ttl.as_millis() as u64)
            .invoke_async(&mut connection)
            .await?;
        Ok(renewed == 1)
    }

    async fn release_lease(
        &self,
        player: Uuid,
        lease: &PlayerLease,
    ) -> Result<(), CoordinationError> {
        let script = redis::Script::new("if redis.call('GET', KEYS[1]) == ARGV[1] then return redis.call('DEL', KEYS[1]) else return 0 end");
        let mut connection = self.connection().await?;
        let _: i64 = script
            .key(lease_key(player))
            .arg(lease_value(lease))
            .invoke_async(&mut connection)
            .await?;
        Ok(())
    }

    async fn allow_command(
        &self,
        token_hash: SessionTokenHash,
        limit: u32,
        window: Duration,
    ) -> Result<bool, CoordinationError> {
        let script = redis::Script::new("local n = redis.call('INCR', KEYS[1]); if n == 1 then redis.call('PEXPIRE', KEYS[1], ARGV[1]) end; return n");
        let mut connection = self.connection().await?;
        let count: u32 = script
            .key(format!("eh:rate:{}", token_hash.cache_key_suffix()))
            .arg(window.as_millis() as u64)
            .invoke_async(&mut connection)
            .await?;
        Ok(count <= limit)
    }

    async fn is_ready(&self) -> bool {
        match self.connection().await {
            Ok(mut connection) => redis::cmd("PING")
                .query_async::<String>(&mut connection)
                .await
                .is_ok(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_cache_resolves_hashed_tokens() {
        let coordinator = InMemorySessionCoordinator::default();
        let ttl = Duration::from_secs(60);
        let token_hash = SessionTokenHash::from_token(Uuid::new_v4());
        let player = Uuid::new_v4();
        coordinator
            .cache_session(token_hash, player, ttl)
            .await
            .unwrap();
        assert_eq!(
            coordinator.resolve(token_hash, ttl).await.unwrap(),
            Some(player)
        );
    }

    #[tokio::test]
    async fn lease_is_single_owner_and_fence_increases() {
        let coordinator = InMemorySessionCoordinator::default();
        let player = Uuid::new_v4();
        let ttl = Duration::from_secs(10);
        let first = coordinator
            .acquire_lease(player, Uuid::new_v4(), ttl)
            .await
            .unwrap()
            .unwrap();
        assert!(coordinator
            .acquire_lease(player, Uuid::new_v4(), ttl)
            .await
            .unwrap()
            .is_none());
        coordinator.release_lease(player, &first).await.unwrap();
        let second = coordinator
            .acquire_lease(player, Uuid::new_v4(), ttl)
            .await
            .unwrap()
            .unwrap();
        assert!(second.fence > first.fence);
    }

    #[tokio::test]
    async fn command_budget_rejects_excess() {
        let coordinator = InMemorySessionCoordinator::default();
        let token_hash = SessionTokenHash::from_token(Uuid::new_v4());
        let window = Duration::from_secs(1);
        assert!(coordinator
            .allow_command(token_hash, 2, window)
            .await
            .unwrap());
        assert!(coordinator
            .allow_command(token_hash, 2, window)
            .await
            .unwrap());
        assert!(!coordinator
            .allow_command(token_hash, 2, window)
            .await
            .unwrap());
    }
}
