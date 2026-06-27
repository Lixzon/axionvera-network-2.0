use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

use crate::database::ConnectionPool;
use crate::error::NetworkError;
use crate::soroban_rpc_client::SorobanRpcClient;
use crate::stellar_service::StellarService;

#[derive(Debug, Serialize, Deserialize)]
struct GetEventsParams {
    #[serde(rename = "startLedger")]
    start_ledger: u32,
    filters: Vec<EventFilter>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventFilter {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "contractIds")]
    contract_ids: Vec<String>,
    topics: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EventsResult {
    events: Vec<SorobanEvent>,
    #[serde(rename = "latestLedger")]
    latest_ledger: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SorobanEvent {
    #[serde(rename = "type")]
    event_type: String,
    ledger: u32,
    #[serde(rename = "contractId")]
    contract_id: String,
    id: String,
    topic: Vec<String>,
    value: SorobanEventValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct SorobanEventValue {
    xdr: String,
}

pub struct EventIndexer {
    stellar_service: Arc<StellarService>,
    connection_pool: ConnectionPool,
    soroban_rpc_client: Arc<SorobanRpcClient>,
    contract_id: String,
    polling_interval_secs: u64,
}

impl EventIndexer {
    pub fn new(
        stellar_service: Arc<StellarService>,
        connection_pool: ConnectionPool,
        soroban_rpc_client: Arc<SorobanRpcClient>,
        contract_id: String,
        polling_interval_secs: u64,
    ) -> Self {
        Self {
            stellar_service,
            connection_pool,
            soroban_rpc_client,
            contract_id,
            polling_interval_secs,
        }
    }

    #[instrument(skip(self))]
    pub async fn start(&self, shutdown_token: CancellationToken) -> Result<(), NetworkError> {
        info!(
            "Starting Soroban Event Indexer for contract: {}",
            self.contract_id
        );

        let mut interval = time::interval(Duration::from_secs(self.polling_interval_secs));
        let mut current_ledger = self.connection_pool.get_last_processed_ledger().await?;

        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    info!("Event indexer stopped gracefully");
                    return Ok(());
                }
                _ = interval.tick() => {}
            }

            let filter = EventFilter {
                event_type: "contract".to_string(),
                contract_ids: vec![self.contract_id.clone()],
                topics: vec![vec!["AxVault".to_string()]],
            };

            let params = GetEventsParams {
                start_ledger: current_ledger,
                filters: vec![filter],
            };

            match self
                .soroban_rpc_client
                .call::<_, EventsResult>("getEvents", params)
                .await
            {
                Ok(events_result) => {
                    for event in events_result.events {
                        info!(
                            event_id = %event.id,
                            ledger = event.ledger,
                            "Processing AxionVault Soroban event"
                        );

                        let protocol = event.topic.get(0).cloned();
                        let action = event.topic.get(1).cloned();
                        
                        // Include event schema version in the JSON payload.
                        // The database stores event_version in the data field
                        // for forward compatibility.
                        let event_json = serde_json::json!({
                            "id": event.id,
                            "ledger": event.ledger,
                            "contract_id": event.contract_id,
                            "topic": event.topic,
                            "value": event.value,
                            "event_version": 1,
                        });

                        self.connection_pool
                            .insert_event(
                                &event.id,
                                event.ledger,
                                &event.contract_id,
                                &event.event_type,
                                protocol.as_deref(),
                                action.as_deref(),
                                None,
                                None,
                                None,
                                None,
                                event_json,
                            )
                            .await?;
                    }

                    current_ledger = events_result.latest_ledger + 1;
                    self.connection_pool.update_last_processed_ledger(current_ledger).await?;
                }
                Err(e) => {
                    error!(error = %e, "Failed to fetch Soroban events");
                }
            }
        }
    }
}
