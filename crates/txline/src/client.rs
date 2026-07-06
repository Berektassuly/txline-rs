//! Top-level TxLINE client.

use std::sync::{Arc, RwLock};

use reqwest::{Method, Response, StatusCode, Url};
use serde::{Serialize, de::DeserializeOwned};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::Mutex;

use crate::auth::{
    ActivationPayload, ApiToken, AuthHeaders, GuestJwt, GuestSession, TokenResponse,
    activation_preimage,
};
use crate::config::TxlineConfig;
use crate::http::{fixtures::FixturesClient, odds::OddsClient, scores::ScoresClient};
use crate::solana::SolanaClient;
use crate::solana::transaction_safety::ValidatedPurchaseQuote;
use crate::stream::{odds::OddsStreamClient, scores::ScoresStreamClient};
use crate::{Result, TxlineError};

#[derive(Debug, Default, Clone)]
struct TokenState {
    guest_jwt: Option<GuestJwt>,
    api_token: Option<ApiToken>,
}

/// Entry point for TxLINE Devnet operations.
#[derive(Debug, Clone)]
pub struct TxlineClient {
    config: TxlineConfig,
    http: reqwest::Client,
    tokens: Arc<RwLock<TokenState>>,
    refresh_lock: Arc<Mutex<()>>,
}

impl TxlineClient {
    /// Create a Devnet client from configuration.
    pub fn new(config: TxlineConfig) -> Result<Self> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .user_agent(format!("txline/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            config,
            http,
            tokens: Arc::new(RwLock::new(TokenState::default())),
            refresh_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn config(&self) -> &TxlineConfig {
        &self.config
    }

    pub fn fixtures(&self) -> FixturesClient<'_> {
        FixturesClient::new(self)
    }

    pub fn odds(&self) -> OddsClient<'_> {
        OddsClient::new(self)
    }

    pub fn scores(&self) -> ScoresClient<'_> {
        ScoresClient::new(self)
    }

    pub fn odds_stream(&self) -> OddsStreamClient {
        OddsStreamClient::new(self.clone())
    }

    pub fn scores_stream(&self) -> ScoresStreamClient {
        ScoresStreamClient::new(self.clone())
    }

    pub fn solana(&self) -> SolanaClient<'_> {
        SolanaClient::new(&self.config)
    }

    /// Fetch a raw purchase quote response from the backend.
    ///
    /// For signing or submission flows, prefer
    /// [`TxlineClient::purchase_quote_checked`], which validates the quote
    /// transaction before exposing transaction bytes.
    pub async fn purchase_quote(
        &self,
        buyer_pubkey: impl Into<String>,
        txline_amount: u64,
    ) -> Result<crate::http::models::PurchaseQuoteResponse> {
        crate::solana::purchase::purchase_quote(self, buyer_pubkey, txline_amount).await
    }

    /// Fetch and validate a purchase quote before returning transaction bytes.
    pub async fn purchase_quote_checked(
        &self,
        buyer: Pubkey,
        txline_amount: u64,
        expected_backend_signer: Pubkey,
    ) -> Result<ValidatedPurchaseQuote> {
        crate::solana::purchase::purchase_quote_checked(
            self,
            buyer,
            txline_amount,
            expected_backend_signer,
        )
        .await
    }

    /// Acquire and store a fresh Devnet guest JWT.
    pub async fn start_guest_session(&self) -> Result<GuestSession> {
        let _guard = self.refresh_lock.lock().await;
        self.start_guest_session_inner().await
    }

    async fn start_guest_session_inner(&self) -> Result<GuestSession> {
        let response = self.http.post(&self.config.guest_auth_url).send().await?;
        let token = Self::decode_response::<TokenResponse>(response)
            .await?
            .token;
        let token = GuestJwt::new(token)?;
        self.set_guest_jwt(token.clone());
        Ok(GuestSession { token })
    }

    async fn refresh_guest_session_after_failure(
        &self,
        stale_jwt: Option<GuestJwt>,
    ) -> Result<GuestSession> {
        let _guard = self.refresh_lock.lock().await;
        if let Some(stale_jwt) = stale_jwt
            && let Some(current) = self.guest_jwt()
            && current != stale_jwt
        {
            return Ok(GuestSession { token: current });
        }
        self.start_guest_session_inner().await
    }

    pub fn set_guest_jwt(&self, jwt: GuestJwt) {
        let mut tokens = self.tokens.write().expect("token lock poisoned");
        tokens.guest_jwt = Some(jwt);
    }

    pub fn set_api_token(&self, token: ApiToken) {
        let mut tokens = self.tokens.write().expect("token lock poisoned");
        tokens.api_token = Some(token);
    }

    pub fn guest_jwt(&self) -> Option<GuestJwt> {
        self.tokens
            .read()
            .expect("token lock poisoned")
            .guest_jwt
            .clone()
    }

    pub fn api_token(&self) -> Option<ApiToken> {
        self.tokens
            .read()
            .expect("token lock poisoned")
            .api_token
            .clone()
    }

    pub fn auth_headers(&self, require_api_token: bool) -> Result<AuthHeaders> {
        let tokens = self.tokens.read().expect("token lock poisoned");
        let jwt = tokens
            .guest_jwt
            .clone()
            .ok_or(TxlineError::MissingGuestJwt)?;
        let api_token = if require_api_token {
            Some(
                tokens
                    .api_token
                    .clone()
                    .ok_or(TxlineError::MissingApiToken)?,
            )
        } else {
            tokens.api_token.clone()
        };
        Ok(AuthHeaders::new(jwt, api_token))
    }

    /// Activate an API token after a confirmed Devnet `subscribe` transaction.
    ///
    /// The caller signs [`activation_preimage`] and passes the base64 detached
    /// wallet signature. The SDK sends the stored guest JWT and persists the
    /// returned API token.
    pub async fn activate_subscription(
        &self,
        tx_sig: impl AsRef<str>,
        selected_leagues: &[i32],
        wallet_signature_base64: impl AsRef<str>,
    ) -> Result<ApiToken> {
        let jwt = self.guest_jwt().ok_or(TxlineError::MissingGuestJwt)?;
        let tx_sig = tx_sig.as_ref();
        let wallet_signature_base64 = wallet_signature_base64.as_ref();
        if tx_sig.trim().is_empty() {
            return Err(TxlineError::invalid_input(
                "subscription transaction signature must not be empty",
            ));
        }
        if wallet_signature_base64.trim().is_empty() {
            return Err(TxlineError::invalid_input(
                "wallet activation signature must not be empty",
            ));
        }

        let payload = ActivationPayload {
            tx_sig,
            wallet_signature: wallet_signature_base64,
            leagues: selected_leagues,
        };
        let response = self
            .http
            .post(self.api_url("/token/activate")?)
            .headers(AuthHeaders::new(jwt, None).to_header_map()?)
            .json(&payload)
            .send()
            .await?;
        let token_text = Self::decode_text_response(response).await?;
        let token = if token_text.trim_start().starts_with('{') {
            serde_json::from_str::<TokenResponse>(&token_text)?.token
        } else {
            token_text
        };
        let token = ApiToken::new(token)?;
        self.set_api_token(token.clone());
        Ok(token)
    }

    pub fn activation_preimage(
        &self,
        tx_sig: impl AsRef<str>,
        selected_leagues: &[i32],
    ) -> Result<String> {
        let jwt = self.guest_jwt().ok_or(TxlineError::MissingGuestJwt)?;
        Ok(activation_preimage(tx_sig, selected_leagues, &jwt))
    }

    pub(crate) async fn get_json<T>(
        &self,
        path: &str,
        query: Vec<(&'static str, String)>,
        require_api_token: bool,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.request_json(
            Method::GET,
            path,
            query,
            Option::<&()>::None,
            require_api_token,
        )
        .await
    }

    pub(crate) async fn post_json<B, T>(
        &self,
        path: &str,
        body: &B,
        require_api_token: bool,
    ) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        self.request_json(
            Method::POST,
            path,
            Vec::new(),
            Some(body),
            require_api_token,
        )
        .await
    }

    async fn request_json<B, T>(
        &self,
        method: Method,
        path: &str,
        query: Vec<(&'static str, String)>,
        body: Option<&B>,
        require_api_token: bool,
    ) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let stale_jwt = self.guest_jwt();
        let mut response = self
            .send_request(method.clone(), path, &query, body, require_api_token)
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            self.refresh_guest_session_after_failure(stale_jwt).await?;
            response = self
                .send_request(method, path, &query, body, require_api_token)
                .await?;
        }
        Self::decode_response(response).await
    }

    async fn send_request<B>(
        &self,
        method: Method,
        path: &str,
        query: &[(&'static str, String)],
        body: Option<&B>,
        require_api_token: bool,
    ) -> Result<Response>
    where
        B: Serialize + ?Sized,
    {
        let mut request = self
            .http
            .request(method, self.api_url(path)?)
            .headers(self.auth_headers(require_api_token)?.to_header_map()?);
        if !query.is_empty() {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        Ok(request.send().await?)
    }

    pub(crate) async fn sse_response(
        &self,
        path: &str,
        query: Vec<(&'static str, String)>,
        last_event_id: Option<&str>,
    ) -> Result<Response> {
        let stale_jwt = self.guest_jwt();
        let mut response = self.send_sse_request(path, &query, last_event_id).await?;
        if is_refreshable_sse_status(response.status()) {
            self.refresh_guest_session_after_failure(stale_jwt).await?;
            response = self.send_sse_request(path, &query, last_event_id).await?;
        }
        if !response.status().is_success() {
            return Err(Self::status_error(response).await);
        }
        Ok(response)
    }

    async fn send_sse_request(
        &self,
        path: &str,
        query: &[(&'static str, String)],
        last_event_id: Option<&str>,
    ) -> Result<Response> {
        let mut request = self
            .http
            .get(self.api_url(path)?)
            .headers(self.auth_headers(true)?.to_header_map()?)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .header(reqwest::header::CACHE_CONTROL, "no-cache");
        if let Some(last_event_id) = last_event_id {
            request = request.header("Last-Event-ID", last_event_id);
        }
        if !query.is_empty() {
            request = request.query(query);
        }
        Ok(request.send().await?)
    }

    fn api_url(&self, path: &str) -> Result<Url> {
        let path = path.strip_prefix('/').unwrap_or(path);
        Ok(Url::parse(&format!("{}/{}", self.config.api_base, path))?)
    }

    async fn decode_response<T>(response: Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        if !response.status().is_success() {
            return Err(Self::status_error(response).await);
        }
        Ok(response.json::<T>().await?)
    }

    async fn decode_text_response(response: Response) -> Result<String> {
        if !response.status().is_success() {
            return Err(Self::status_error(response).await);
        }
        Ok(response.text().await?)
    }

    async fn status_error(response: Response) -> TxlineError {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        TxlineError::HttpStatus { status, body }
    }
}

fn is_refreshable_sse_status(status: StatusCode) -> bool {
    status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEVNET_PROGRAM_ID;
    use crate::config::{
        DEVNET_API_HOST, DEVNET_RPC_URL, DEVNET_TXL_MINT, DEVNET_USDT_MINT, Network,
    };
    use crate::solana::pda::parse_pubkey;
    use crate::solana::purchase::{
        devnet_purchase_subscription_token_usdt_accounts,
        purchase_subscription_token_usdt_instruction,
    };
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use solana_sdk::hash::Hash;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_sdk::transaction::{Transaction, VersionedTransaction};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn sse_refreshable_statuses_match_devnet_examples() {
        assert!(is_refreshable_sse_status(StatusCode::UNAUTHORIZED));
        assert!(is_refreshable_sse_status(StatusCode::FORBIDDEN));
        assert!(!is_refreshable_sse_status(StatusCode::NOT_FOUND));
    }

    #[tokio::test]
    async fn rest_401_refreshes_guest_jwt_once() {
        let server = TestServer::spawn(3);
        let client = test_client(&server);
        client.set_guest_jwt(GuestJwt::new("stale").unwrap());

        let value: serde_json::Value = client.get_json("/test", Vec::new(), false).await.unwrap();

        assert_eq!(value["ok"], true);
        assert_eq!(server.auth_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn sse_403_refreshes_guest_jwt() {
        let server = TestServer::spawn(3);
        let client = test_client(&server);
        client.set_guest_jwt(GuestJwt::new("stale").unwrap());
        client.set_api_token(ApiToken::new("api").unwrap());

        let response = client
            .sse_response("/scores/stream", Vec::new(), None)
            .await
            .unwrap();

        assert!(response.status().is_success());
        assert_eq!(server.auth_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_refreshes_share_one_guest_session_request() {
        let server = TestServer::spawn(12);
        let client = test_client(&server);
        let stale = GuestJwt::new("stale").unwrap();
        client.set_guest_jwt(stale.clone());

        let mut handles = Vec::new();
        for _ in 0..10 {
            let client = client.clone();
            let stale = stale.clone();
            handles.push(tokio::spawn(async move {
                client
                    .refresh_guest_session_after_failure(Some(stale))
                    .await
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(server.auth_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn purchase_quote_checked_returns_validated_transaction_bytes() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 1_000);
        let expected_bytes = wincode::serialize(&transaction).unwrap();
        let server = TestServer::spawn_with_quote(1, quote_json(&transaction));
        let client = test_client(&server);
        client.set_guest_jwt(GuestJwt::new("guest").unwrap());

        let quote = client
            .purchase_quote_checked(buyer.pubkey(), 1_000, backend.pubkey())
            .await
            .unwrap();

        assert_eq!(quote.transaction_bytes(), expected_bytes.as_slice());
        assert!(quote.safety_report.backend_signer_present);
        assert_eq!(quote.safety_report.txline_purchase_instruction_count, 1);
    }

    #[tokio::test]
    async fn purchase_quote_checked_rejects_malformed_transaction_bytes() {
        let buyer = Keypair::new();
        let backend = Keypair::new();
        let transaction = signed_purchase_transaction(&buyer, &backend, 999);
        let server = TestServer::spawn_with_quote(1, quote_json(&transaction));
        let client = test_client(&server);
        client.set_guest_jwt(GuestJwt::new("guest").unwrap());

        let err = client
            .purchase_quote_checked(buyer.pubkey(), 1_000, backend.pubkey())
            .await
            .unwrap_err();

        assert!(err.to_string().contains("txline_amount"));
    }

    struct TestServer {
        base_url: String,
        auth_count: Arc<AtomicUsize>,
    }

    impl TestServer {
        fn spawn(max_requests: usize) -> Self {
            Self::spawn_with_optional_quote(max_requests, None)
        }

        fn spawn_with_quote(max_requests: usize, quote_body: String) -> Self {
            Self::spawn_with_optional_quote(max_requests, Some(quote_body))
        }

        fn spawn_with_optional_quote(max_requests: usize, quote_body: Option<String>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let auth_count = Arc::new(AtomicUsize::new(0));
            let rest_count = Arc::new(AtomicUsize::new(0));
            let sse_count = Arc::new(AtomicUsize::new(0));
            let auth_count_for_thread = Arc::clone(&auth_count);
            let rest_count_for_thread = Arc::clone(&rest_count);
            let sse_count_for_thread = Arc::clone(&sse_count);

            thread::spawn(move || {
                for stream in listener.incoming().take(max_requests).flatten() {
                    handle_connection(
                        stream,
                        &auth_count_for_thread,
                        &rest_count_for_thread,
                        &sse_count_for_thread,
                        quote_body.as_deref(),
                    );
                }
            });

            Self {
                base_url,
                auth_count,
            }
        }
    }

    fn handle_connection(
        mut stream: TcpStream,
        auth_count: &AtomicUsize,
        rest_count: &AtomicUsize,
        sse_count: &AtomicUsize,
        quote_body: Option<&str>,
    ) {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while let Ok(read) = stream.read(&mut buf) {
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        read_request_body(&mut stream, &mut request);
        let request = String::from_utf8_lossy(&request);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");
        let (status, content_type, body) = if path == "/auth" {
            auth_count.fetch_add(1, Ordering::SeqCst);
            ("200 OK", "application/json", r#"{"token":"fresh"}"#)
        } else if path == "/api/test" {
            if rest_count.fetch_add(1, Ordering::SeqCst) == 0 {
                ("401 Unauthorized", "text/plain", "expired")
            } else {
                ("200 OK", "application/json", r#"{"ok":true}"#)
            }
        } else if path == "/api/scores/stream" {
            if sse_count.fetch_add(1, Ordering::SeqCst) == 0 {
                ("403 Forbidden", "text/plain", "expired")
            } else {
                ("200 OK", "text/event-stream", "")
            }
        } else if path == "/api/guest/purchase/quote" {
            match quote_body {
                Some(body) => ("200 OK", "application/json", body),
                None => ("500 Internal Server Error", "text/plain", "missing quote"),
            }
        } else {
            ("404 Not Found", "text/plain", "missing")
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    fn read_request_body(stream: &mut TcpStream, request: &mut Vec<u8>) {
        let header_end = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4);
        let Some(header_end) = header_end else {
            return;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let target_len = header_end + content_length;
        let mut buf = [0u8; 1024];
        while request.len() < target_len {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(read) => request.extend_from_slice(&buf[..read]),
            }
        }
    }

    fn signed_purchase_transaction(
        buyer: &Keypair,
        backend: &Keypair,
        amount: u64,
    ) -> VersionedTransaction {
        let program_id = parse_pubkey(DEVNET_PROGRAM_ID).unwrap();
        let accounts =
            devnet_purchase_subscription_token_usdt_accounts(buyer.pubkey(), backend.pubkey())
                .unwrap();
        let purchase_ix =
            purchase_subscription_token_usdt_instruction(program_id, accounts, amount).unwrap();
        let blockhash = Hash::new_unique();
        let mut transaction = Transaction::new_with_payer(&[purchase_ix], Some(&buyer.pubkey()));
        transaction.sign(&[buyer, backend], blockhash);
        VersionedTransaction::from(transaction)
    }

    fn quote_json(transaction: &VersionedTransaction) -> String {
        serde_json::json!({
            "transactionBase64": STANDARD.encode(wincode::serialize(transaction).unwrap()),
            "baseUsdtCost": 1.0,
            "feeUsdtAmount": 0.25,
            "totalUsdtCharged": 1.25,
        })
        .to_string()
    }

    fn test_client(server: &TestServer) -> TxlineClient {
        TxlineClient {
            config: TxlineConfig {
                network: Network::Devnet,
                api_host: DEVNET_API_HOST.to_owned(),
                api_base: format!("{}/api", server.base_url),
                guest_auth_url: format!("{}/auth", server.base_url),
                program_id: DEVNET_PROGRAM_ID.to_owned(),
                txl_mint: DEVNET_TXL_MINT.to_owned(),
                usdt_mint: DEVNET_USDT_MINT.to_owned(),
                rpc_url: DEVNET_RPC_URL.to_owned(),
            },
            http: reqwest::Client::new(),
            tokens: Arc::new(RwLock::new(TokenState::default())),
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }
}
