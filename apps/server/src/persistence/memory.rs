use super::{
    async_trait, new_account_roster, DurablePlayerAggregate, HashMap, HashSet, LoadedPlayerState,
    PendingOperation, PlayerAccountRecord, PlayerRepository, RepositoryError, RwLock,
    SessionTokenHash, Uuid,
};

pub struct InMemoryPlayerRepository {
    identities: RwLock<HashMap<SessionTokenHash, Uuid>>,
    accounts: RwLock<HashMap<String, PlayerAccountRecord>>,
    pub(super) durable: RwLock<MemoryDurableState>,
}

#[derive(Default)]
pub(super) struct MemoryDurableState {
    pub(super) states: HashMap<Uuid, (DurablePlayerAggregate, i64, i64)>,
    pub(super) reward_operations: HashSet<(Uuid, Uuid)>,
    pub(super) command_operations: HashSet<(Uuid, Uuid)>,
}

impl Default for InMemoryPlayerRepository {
    fn default() -> Self {
        Self {
            identities: RwLock::new(HashMap::new()),
            accounts: RwLock::new(HashMap::new()),
            durable: RwLock::new(MemoryDurableState::default()),
        }
    }
}

#[async_trait]
impl PlayerRepository for InMemoryPlayerRepository {
    async fn create_account(
        &self,
        normalized_email: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<PlayerAccountRecord, RepositoryError> {
        let mut accounts = self.accounts.write().await;
        if accounts.contains_key(normalized_email) {
            return Err(RepositoryError::AccountExists);
        }
        let account = PlayerAccountRecord {
            account_id: Uuid::new_v4(),
            player_token: Uuid::new_v4(),
            normalized_email: normalized_email.to_owned(),
            display_name: display_name.to_owned(),
            password_hash: password_hash.to_owned(),
            is_demo: false,
        };
        accounts.insert(normalized_email.to_owned(), account.clone());
        Ok(account)
    }

    async fn find_account_by_email(
        &self,
        normalized_email: &str,
    ) -> Result<Option<PlayerAccountRecord>, RepositoryError> {
        Ok(self.accounts.read().await.get(normalized_email).cloned())
    }

    async fn bind_session(
        &self,
        token_hash: SessionTokenHash,
        player_token: Uuid,
    ) -> Result<(), RepositoryError> {
        self.identities
            .write()
            .await
            .insert(token_hash, player_token);
        Ok(())
    }

    async fn resolve_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Option<Uuid>, RepositoryError> {
        Ok(self.identities.read().await.get(&token_hash).copied())
    }

    async fn resolve_or_create_local_identity(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Uuid, RepositoryError> {
        let mut identities = self.identities.write().await;
        Ok(*identities.entry(token_hash).or_insert_with(Uuid::new_v4))
    }

    async fn load_or_create(
        &self,
        player_token: Uuid,
    ) -> Result<LoadedPlayerState, RepositoryError> {
        let mut durable = self.durable.write().await;
        let entry = durable.states.entry(player_token).or_insert_with(|| {
            let mut state = DurablePlayerAggregate::default();
            state.buildings.town_gold = 100_000;
            state.hunter_roster = new_account_roster(player_token);
            (state, 0, 0)
        });
        let (state, revision, _) = entry;
        Ok(LoadedPlayerState {
            state: state.clone(),
            revision: *revision,
        })
    }

    async fn persist(
        &self,
        player_token: Uuid,
        state: &DurablePlayerAggregate,
        expected_revision: i64,
        lease_fence: i64,
        operations: &[PendingOperation],
    ) -> Result<i64, RepositoryError> {
        let mut durable = self.durable.write().await;
        let entry = durable.states.entry(player_token).or_default();
        if entry.1 != expected_revision || entry.2 > lease_fence {
            return Err(RepositoryError::RevisionConflict);
        }
        entry.0 = state.clone();
        entry.1 += 1;
        entry.2 = lease_fence;
        let next_revision = entry.1;
        for operation in operations {
            match operation {
                PendingOperation::Reward { operation_id, .. } => {
                    durable
                        .reward_operations
                        .insert((player_token, *operation_id));
                }
                PendingOperation::Equip { command_id, .. } => {
                    durable
                        .command_operations
                        .insert((player_token, *command_id));
                }
            }
        }
        Ok(next_revision)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
