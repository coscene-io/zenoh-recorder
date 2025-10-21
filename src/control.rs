// Copyright 2025 coScene
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};
use zenoh::prelude::r#async::*;
use zenoh::queryable::Query;
use zenoh::Session;

use crate::protocol::{RecorderCommand, RecorderRequest, RecorderResponse, StatusResponse};
use crate::recorder::RecorderManager;

/// Control interface for handling recorder commands via Zenoh queryable
pub struct ControlInterface {
    session: Arc<Session>,
    recorder_manager: Arc<RecorderManager>,
    device_id: String,
}

impl ControlInterface {
    pub fn new(
        session: Arc<Session>,
        recorder_manager: Arc<RecorderManager>,
        device_id: String,
    ) -> Self {
        Self {
            session,
            recorder_manager,
            device_id,
        }
    }

    /// Run the control interface (blocks until stopped)
    pub async fn run(&self) -> Result<()> {
        // Declare queryable for control commands
        let control_key = format!("recorder/control/{}", self.device_id);
        let queryable = self
            .session
            .declare_queryable(&control_key)
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        info!("Control interface listening on '{}'", control_key);

        // Declare queryable for status queries
        let status_key = "recorder/status/**";
        let status_queryable = self
            .session
            .declare_queryable(status_key)
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        info!("Status interface listening on '{}'", status_key);

        // Handle queries in parallel
        loop {
            tokio::select! {
                Ok(query) = queryable.recv_async() => {
                    let recorder_manager = self.recorder_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_control_query(query, recorder_manager).await {
                            error!("Error handling control query: {}", e);
                        }
                    });
                }
                Ok(query) = status_queryable.recv_async() => {
                    let recorder_manager = self.recorder_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_status_query(query, recorder_manager).await {
                            error!("Error handling status query: {}", e);
                        }
                    });
                }
            }
        }
    }

    async fn handle_control_query(
        query: Query,
        recorder_manager: Arc<RecorderManager>,
    ) -> Result<()> {
        info!("Received control query on '{}'", query.selector());

        // Parse request from query value (payload is in query.value().payload in v0.11)
        let request: RecorderRequest = if let Some(value) = query.value() {
            let bytes = value.payload.contiguous();
            serde_json::from_slice(&bytes)?
        } else {
            let response = RecorderResponse::error("Missing request payload".to_string());
            let response_bytes = serde_json::to_vec(&response)?;
            query
                .reply(Ok(Sample::new(query.key_expr().clone(), response_bytes)))
                .res()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            return Ok(());
        };

        info!("Processing command: {:?}", request.command);

        // Handle the command
        let response = match request.command {
            RecorderCommand::Start => recorder_manager.start_recording(request).await,
            RecorderCommand::Pause => {
                recorder_manager
                    .pause_recording(&request.recording_id.unwrap_or_default())
                    .await
            }
            RecorderCommand::Resume => {
                recorder_manager
                    .resume_recording(&request.recording_id.unwrap_or_default())
                    .await
            }
            RecorderCommand::Cancel => {
                recorder_manager
                    .cancel_recording(&request.recording_id.unwrap_or_default())
                    .await
            }
            RecorderCommand::Finish => {
                recorder_manager
                    .finish_recording(&request.recording_id.unwrap_or_default())
                    .await
            }
        };

        // Send response
        let response_bytes = serde_json::to_vec(&response)?;
        query
            .reply(Ok(Sample::new(query.key_expr().clone(), response_bytes)))
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    async fn handle_status_query(
        query: Query,
        recorder_manager: Arc<RecorderManager>,
    ) -> Result<()> {
        info!("Received status query on '{}'", query.selector());

        // Extract recording_id from key expression
        // Pattern: recorder/status/{recording_id}
        let key_parts: Vec<&str> = query.key_expr().as_str().split('/').collect();
        if key_parts.len() < 3 {
            let response = StatusResponse {
                success: false,
                message: "Invalid status query format".to_string(),
                status: crate::protocol::RecordingStatus::Idle,
                scene: None,
                skills: vec![],
                organization: None,
                task_id: None,
                device_id: String::new(),
                data_collector_id: None,
                active_topics: vec![],
                buffer_size_bytes: 0,
                total_recorded_bytes: 0,
            };
            let response_bytes = serde_json::to_vec(&response)?;
            query
                .reply(Ok(Sample::new(query.key_expr().clone(), response_bytes)))
                .res()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            return Ok(());
        }

        let recording_id = key_parts[2];

        // Get status
        let response = recorder_manager.get_status(recording_id).await;

        // Send response
        let response_bytes = serde_json::to_vec(&response)?;
        query
            .reply(Ok(Sample::new(query.key_expr().clone(), response_bytes)))
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }
}
