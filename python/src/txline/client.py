"""Top-level TxLINE REST clients."""

from __future__ import annotations

import asyncio
import threading
from typing import Any
from urllib.parse import urljoin

import httpx

from txline.auth import ApiToken, AuthHeaders, GuestJwt, activation_preimage
from txline.config import TxlineConfig
from txline.errors import (
    HttpStatusError,
    InvalidInputError,
    MissingApiTokenError,
    MissingGuestJwtError,
)
from txline.fixtures import AsyncFixturesClient, FixturesClient
from txline.odds import AsyncOddsClient, OddsClient
from txline.purchase import PurchaseQuoteResponse, validate_quote_amount
from txline.scores import AsyncScoresClient, ScoresClient


class _TokenState:
    def __init__(self) -> None:
        self.guest_jwt: GuestJwt | None = None
        self.api_token: ApiToken | None = None


class TxlineClient:
    """Reusable synchronous TxLINE client."""

    def __init__(self, config: TxlineConfig | None = None, http_client: httpx.Client | None = None):
        self.config = config or TxlineConfig.devnet()
        self.config.validate()
        self._http = http_client or httpx.Client(
            headers={"User-Agent": "txline-python/0.1.0"}, timeout=30.0
        )
        self._owns_http = http_client is None
        self._tokens = _TokenState()
        self._token_lock = threading.RLock()
        self._refresh_lock = threading.RLock()

    def __enter__(self) -> TxlineClient:
        return self

    def __exit__(self, *_exc: object) -> None:
        self.close()

    def close(self) -> None:
        if self._owns_http:
            self._http.close()

    def fixtures(self) -> FixturesClient:
        return FixturesClient(self)

    def odds(self) -> OddsClient:
        return OddsClient(self)

    def scores(self) -> ScoresClient:
        return ScoresClient(self)

    def odds_stream(self) -> Any:
        from txline.sse import SyncSseStreamClient

        return SyncSseStreamClient(self, "/odds/stream")

    def scores_stream(self) -> Any:
        from txline.sse import SyncSseStreamClient

        return SyncSseStreamClient(self, "/scores/stream")

    def solana(self) -> Any:
        from txline.solana import SolanaClient

        return SolanaClient(self.config)

    def set_guest_jwt(self, jwt: GuestJwt) -> None:
        with self._token_lock:
            self._tokens.guest_jwt = jwt

    def set_api_token(self, token: ApiToken) -> None:
        with self._token_lock:
            self._tokens.api_token = token

    def guest_jwt(self) -> GuestJwt | None:
        with self._token_lock:
            return self._tokens.guest_jwt

    def api_token(self) -> ApiToken | None:
        with self._token_lock:
            return self._tokens.api_token

    def auth_headers(self, require_api_token: bool) -> AuthHeaders:
        with self._token_lock:
            guest_jwt = self._tokens.guest_jwt
            api_token = self._tokens.api_token
        if guest_jwt is None:
            raise MissingGuestJwtError()
        if require_api_token and api_token is None:
            raise MissingApiTokenError()
        return AuthHeaders(guest_jwt, api_token if require_api_token else api_token)

    def start_guest_session(self) -> GuestJwt:
        with self._refresh_lock:
            return self._start_guest_session_inner()

    def _start_guest_session_inner(self) -> GuestJwt:
        response = self._http.post(self.config.guest_auth_url)
        payload = _decode_response(response)
        jwt = GuestJwt(str(payload["token"]))
        self.set_guest_jwt(jwt)
        return jwt

    def activate_subscription(
        self,
        tx_sig: str,
        selected_leagues: list[int] | tuple[int, ...],
        wallet_signature_base64: str,
    ) -> ApiToken:
        jwt = self.guest_jwt()
        if jwt is None:
            raise MissingGuestJwtError()
        if not tx_sig.strip():
            raise InvalidInputError("subscription transaction signature must not be empty")
        if not wallet_signature_base64.strip():
            raise InvalidInputError("wallet activation signature must not be empty")
        response = self._http.post(
            self._api_url("/token/activate"),
            headers=AuthHeaders(jwt).to_headers(),
            json={
                "txSig": tx_sig,
                "walletSignature": wallet_signature_base64,
                "leagues": list(selected_leagues),
            },
        )
        text = _decode_text_response(response)
        if text.lstrip().startswith("{"):
            token_value = str(httpx.Response(200, content=text).json()["token"])
        else:
            token_value = text
        token = ApiToken(token_value)
        self.set_api_token(token)
        return token

    def activation_preimage(
        self, tx_sig: str, selected_leagues: list[int] | tuple[int, ...]
    ) -> str:
        jwt = self.guest_jwt()
        if jwt is None:
            raise MissingGuestJwtError()
        return activation_preimage(tx_sig, selected_leagues, jwt)

    def purchase_quote(self, buyer_pubkey: str, txline_amount: int) -> PurchaseQuoteResponse:
        validate_quote_amount(txline_amount)
        data = self._post_json(
            "/guest/purchase/quote",
            {"buyerPubkey": buyer_pubkey, "txlineAmount": txline_amount},
            False,
        )
        return PurchaseQuoteResponse.from_dict(data)

    def purchase_quote_checked(
        self, buyer: str, txline_amount: int, expected_backend_signer: str
    ) -> Any:
        from txline.solana.transaction_safety import (
            PurchaseTransactionSafetyConfig,
            ValidatedPurchaseQuote,
        )

        quote = self.purchase_quote(buyer, txline_amount)
        config = PurchaseTransactionSafetyConfig.devnet(
            self.config, buyer, txline_amount, expected_backend_signer
        )
        return ValidatedPurchaseQuote.new(quote, config)

    def _get_json(self, path: str, query: list[tuple[str, str]], require_api_token: bool) -> Any:
        stale_jwt = self.guest_jwt()
        response = self._send_request("GET", path, query, None, require_api_token)
        if response.status_code == 401:
            self._refresh_guest_session_after_failure(stale_jwt)
            response = self._send_request("GET", path, query, None, require_api_token)
        return _decode_response(response)

    def _post_json(self, path: str, body: Any, require_api_token: bool) -> Any:
        stale_jwt = self.guest_jwt()
        response = self._send_request("POST", path, [], body, require_api_token)
        if response.status_code == 401:
            self._refresh_guest_session_after_failure(stale_jwt)
            response = self._send_request("POST", path, [], body, require_api_token)
        return _decode_response(response)

    def _send_request(
        self,
        method: str,
        path: str,
        query: list[tuple[str, str]],
        body: Any,
        require_api_token: bool,
    ) -> httpx.Response:
        kwargs: dict[str, Any] = {
            "headers": self.auth_headers(require_api_token).to_headers(),
            "params": query,
        }
        if body is not None:
            kwargs["json"] = body
        return self._http.request(method, self._api_url(path), **kwargs)

    def _refresh_guest_session_after_failure(self, stale_jwt: GuestJwt | None) -> GuestJwt:
        with self._refresh_lock:
            current = self.guest_jwt()
            if stale_jwt is not None and current is not None and current != stale_jwt:
                return current
            return self._start_guest_session_inner()

    def _api_url(self, path: str) -> str:
        return urljoin(self.config.api_base.rstrip("/") + "/", path.lstrip("/"))


class AsyncTxlineClient:
    """Reusable asynchronous TxLINE client."""

    def __init__(
        self, config: TxlineConfig | None = None, http_client: httpx.AsyncClient | None = None
    ):
        self.config = config or TxlineConfig.devnet()
        self.config.validate()
        self._http = http_client or httpx.AsyncClient(
            headers={"User-Agent": "txline-python/0.1.0"}, timeout=30.0
        )
        self._owns_http = http_client is None
        self._tokens = _TokenState()
        self._token_lock = threading.RLock()
        self._refresh_lock = asyncio.Lock()

    async def __aenter__(self) -> AsyncTxlineClient:
        return self

    async def __aexit__(self, *_exc: object) -> None:
        await self.aclose()

    async def aclose(self) -> None:
        if self._owns_http:
            await self._http.aclose()

    def fixtures(self) -> AsyncFixturesClient:
        return AsyncFixturesClient(self)

    def odds(self) -> AsyncOddsClient:
        return AsyncOddsClient(self)

    def scores(self) -> AsyncScoresClient:
        return AsyncScoresClient(self)

    def odds_stream(self) -> Any:
        from txline.sse import AsyncSseStreamClient

        return AsyncSseStreamClient(self, "/odds/stream")

    def scores_stream(self) -> Any:
        from txline.sse import AsyncSseStreamClient

        return AsyncSseStreamClient(self, "/scores/stream")

    def solana(self) -> Any:
        from txline.solana import SolanaClient

        return SolanaClient(self.config)

    def set_guest_jwt(self, jwt: GuestJwt) -> None:
        with self._token_lock:
            self._tokens.guest_jwt = jwt

    def set_api_token(self, token: ApiToken) -> None:
        with self._token_lock:
            self._tokens.api_token = token

    def guest_jwt(self) -> GuestJwt | None:
        with self._token_lock:
            return self._tokens.guest_jwt

    def api_token(self) -> ApiToken | None:
        with self._token_lock:
            return self._tokens.api_token

    def auth_headers(self, require_api_token: bool) -> AuthHeaders:
        with self._token_lock:
            guest_jwt = self._tokens.guest_jwt
            api_token = self._tokens.api_token
        if guest_jwt is None:
            raise MissingGuestJwtError()
        if require_api_token and api_token is None:
            raise MissingApiTokenError()
        return AuthHeaders(guest_jwt, api_token if require_api_token else api_token)

    async def start_guest_session(self) -> GuestJwt:
        async with self._refresh_lock:
            return await self._start_guest_session_inner()

    async def _start_guest_session_inner(self) -> GuestJwt:
        response = await self._http.post(self.config.guest_auth_url)
        payload = _decode_response(response)
        jwt = GuestJwt(str(payload["token"]))
        self.set_guest_jwt(jwt)
        return jwt

    async def activate_subscription(
        self,
        tx_sig: str,
        selected_leagues: list[int] | tuple[int, ...],
        wallet_signature_base64: str,
    ) -> ApiToken:
        jwt = self.guest_jwt()
        if jwt is None:
            raise MissingGuestJwtError()
        if not tx_sig.strip():
            raise InvalidInputError("subscription transaction signature must not be empty")
        if not wallet_signature_base64.strip():
            raise InvalidInputError("wallet activation signature must not be empty")
        response = await self._http.post(
            self._api_url("/token/activate"),
            headers=AuthHeaders(jwt).to_headers(),
            json={
                "txSig": tx_sig,
                "walletSignature": wallet_signature_base64,
                "leagues": list(selected_leagues),
            },
        )
        text = _decode_text_response(response)
        if text.lstrip().startswith("{"):
            token_value = str(httpx.Response(200, content=text).json()["token"])
        else:
            token_value = text
        token = ApiToken(token_value)
        self.set_api_token(token)
        return token

    def activation_preimage(
        self, tx_sig: str, selected_leagues: list[int] | tuple[int, ...]
    ) -> str:
        jwt = self.guest_jwt()
        if jwt is None:
            raise MissingGuestJwtError()
        return activation_preimage(tx_sig, selected_leagues, jwt)

    async def purchase_quote(self, buyer_pubkey: str, txline_amount: int) -> PurchaseQuoteResponse:
        validate_quote_amount(txline_amount)
        data = await self._post_json(
            "/guest/purchase/quote",
            {"buyerPubkey": buyer_pubkey, "txlineAmount": txline_amount},
            False,
        )
        return PurchaseQuoteResponse.from_dict(data)

    async def purchase_quote_checked(
        self, buyer: str, txline_amount: int, expected_backend_signer: str
    ) -> Any:
        from txline.solana.transaction_safety import (
            PurchaseTransactionSafetyConfig,
            ValidatedPurchaseQuote,
        )

        quote = await self.purchase_quote(buyer, txline_amount)
        config = PurchaseTransactionSafetyConfig.devnet(
            self.config, buyer, txline_amount, expected_backend_signer
        )
        return ValidatedPurchaseQuote.new(quote, config)

    async def _get_json(
        self, path: str, query: list[tuple[str, str]], require_api_token: bool
    ) -> Any:
        stale_jwt = self.guest_jwt()
        response = await self._send_request("GET", path, query, None, require_api_token)
        if response.status_code == 401:
            await self._refresh_guest_session_after_failure(stale_jwt)
            response = await self._send_request("GET", path, query, None, require_api_token)
        return _decode_response(response)

    async def _post_json(self, path: str, body: Any, require_api_token: bool) -> Any:
        stale_jwt = self.guest_jwt()
        response = await self._send_request("POST", path, [], body, require_api_token)
        if response.status_code == 401:
            await self._refresh_guest_session_after_failure(stale_jwt)
            response = await self._send_request("POST", path, [], body, require_api_token)
        return _decode_response(response)

    async def _send_request(
        self,
        method: str,
        path: str,
        query: list[tuple[str, str]],
        body: Any,
        require_api_token: bool,
    ) -> httpx.Response:
        kwargs: dict[str, Any] = {
            "headers": self.auth_headers(require_api_token).to_headers(),
            "params": query,
        }
        if body is not None:
            kwargs["json"] = body
        return await self._http.request(method, self._api_url(path), **kwargs)

    async def _refresh_guest_session_after_failure(self, stale_jwt: GuestJwt | None) -> GuestJwt:
        async with self._refresh_lock:
            current = self.guest_jwt()
            if stale_jwt is not None and current is not None and current != stale_jwt:
                return current
            return await self._start_guest_session_inner()

    def _api_url(self, path: str) -> str:
        return urljoin(self.config.api_base.rstrip("/") + "/", path.lstrip("/"))


def _decode_response(response: httpx.Response) -> Any:
    if not 200 <= response.status_code <= 299:
        raise HttpStatusError(response.status_code, response.content)
    return response.json()


def _decode_text_response(response: httpx.Response) -> str:
    if not 200 <= response.status_code <= 299:
        raise HttpStatusError(response.status_code, response.content)
    return response.text
