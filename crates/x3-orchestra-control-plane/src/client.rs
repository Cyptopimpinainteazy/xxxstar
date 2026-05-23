use crate::types::{
    ApprovalCase, DispatchEvidenceRequest, EvidenceBundle, Intent, IntentDispatchRequest,
    NewApprovalCase, NewIntent, NewRewardAccrual, NewVoteReceipt, NewVoteWindow, RewardAccrual,
    VoteReceipt, VoteTally, VoteWindow,
};
use anyhow::{anyhow, Context};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone)]
pub struct ControlPlaneClient {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VoteWindowClosureResponse {
    pub vote_window: VoteWindow,
    pub approval_case: ApprovalCase,
    pub evidence: EvidenceBundle,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct IntentDispatchResponse {
    pub intent: Intent,
    pub evidence: EvidenceBundle,
}

impl ControlPlaneClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            auth_token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn create_intent(&self, input: &NewIntent) -> anyhow::Result<Intent> {
        self.post_json("/intents", input).await
    }

    pub async fn create_approval_case(
        &self,
        input: &NewApprovalCase,
    ) -> anyhow::Result<ApprovalCase> {
        self.post_json("/approval-cases", input).await
    }

    pub async fn open_vote_window(&self, input: &NewVoteWindow) -> anyhow::Result<VoteWindow> {
        self.post_json("/vote-windows", input).await
    }

    pub async fn record_vote(
        &self,
        window_id: &str,
        input: &NewVoteReceipt,
    ) -> anyhow::Result<VoteReceipt> {
        self.post_json(&format!("/vote-windows/{window_id}/votes"), input)
            .await
    }

    pub async fn close_vote_window(
        &self,
        window_id: &str,
    ) -> anyhow::Result<VoteWindowClosureResponse> {
        self.post_json::<VoteWindowClosureResponse, serde_json::Value>(
            &format!("/vote-windows/{window_id}/close"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn import_vote_tally(&self, window_id: &str) -> anyhow::Result<VoteTally> {
        self.post_json::<VoteTally, serde_json::Value>(
            &format!("/vote-windows/{window_id}/imported-tally"),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn dispatch_intent(
        &self,
        intent_id: &str,
        evidence: DispatchEvidenceRequest,
    ) -> anyhow::Result<IntentDispatchResponse> {
        self.post_json(
            &format!("/intents/{intent_id}/dispatch"),
            &IntentDispatchRequest { evidence },
        )
        .await
    }

    pub async fn accrue_reward(&self, input: &NewRewardAccrual) -> anyhow::Result<RewardAccrual> {
        self.post_json("/rewards", input).await
    }

    pub async fn get_evidence_bundle(&self, bundle_id: &str) -> anyhow::Result<EvidenceBundle> {
        self.get_json(&format!("/evidence/{bundle_id}")).await
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let request = self.request(self.client.get(format!("{}{}", self.base_url, path)));
        self.send(request).await
    }

    async fn post_json<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        let request = self.request(
            self.client
                .post(format!("{}{}", self.base_url, path))
                .json(body),
        );
        self.send(request).await
    }

    fn request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_token {
            Some(token) => builder.bearer_auth(token),
            None => builder,
        }
    }

    async fn send<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<T> {
        let response = request
            .send()
            .await
            .context("control-plane request failed")?;
        let status = response.status();
        match response.error_for_status() {
            Ok(ok) => Ok(ok
                .json::<T>()
                .await
                .context("invalid control-plane response body")?),
            Err(err) => Err(anyhow!(
                "control-plane request failed with status {status}: {err}"
            )),
        }
    }
}
