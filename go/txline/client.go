package txline

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/url"
	"path"
	"strconv"
	"strings"
	"sync"
)

type Client struct {
	config Config
	http   *http.Client

	mu          sync.RWMutex
	guestJWT    *GuestJWT
	apiToken    *APIToken
	refreshLock sync.Mutex
}

type ClientOption func(*Client)

func WithHTTPClient(httpClient *http.Client) ClientOption {
	return func(c *Client) {
		if httpClient != nil {
			c.http = httpClient
		}
	}
}

func NewClient(config Config, opts ...ClientOption) (*Client, error) {
	if err := config.validate(); err != nil {
		return nil, err
	}
	client := &Client{config: config, http: http.DefaultClient}
	for _, opt := range opts {
		opt(client)
	}
	return client, nil
}

func newUncheckedClient(config Config, httpClient *http.Client) *Client {
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	return &Client{config: config, http: httpClient}
}

func (c *Client) Config() Config {
	return c.config
}

func (c *Client) SetGuestJWT(jwt GuestJWT) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.guestJWT = &jwt
}

func (c *Client) SetAPIToken(token APIToken) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.apiToken = &token
}

func (c *Client) GuestJWT() (GuestJWT, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	if c.guestJWT == nil {
		return GuestJWT{}, false
	}
	return *c.guestJWT, true
}

func (c *Client) APIToken() (APIToken, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	if c.apiToken == nil {
		return APIToken{}, false
	}
	return *c.apiToken, true
}

func (c *Client) AuthHeaders(requireAPIToken bool) (AuthHeaders, error) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	if c.guestJWT == nil {
		return AuthHeaders{}, ErrMissingGuestJWT
	}
	var apiToken *APIToken
	if requireAPIToken {
		if c.apiToken == nil {
			return AuthHeaders{}, ErrMissingAPIToken
		}
		copy := *c.apiToken
		apiToken = &copy
	} else if c.apiToken != nil {
		copy := *c.apiToken
		apiToken = &copy
	}
	return AuthHeaders{GuestJWT: *c.guestJWT, APIToken: apiToken}, nil
}

func (c *Client) StartGuestSession(ctx context.Context) (GuestSession, error) {
	c.refreshLock.Lock()
	defer c.refreshLock.Unlock()
	return c.startGuestSessionLocked(ctx)
}

func (c *Client) startGuestSessionLocked(ctx context.Context) (GuestSession, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.config.GuestAuthURL, nil)
	if err != nil {
		return GuestSession{}, err
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return GuestSession{}, err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return GuestSession{}, statusError(resp)
	}
	var payload tokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&payload); err != nil {
		return GuestSession{}, err
	}
	jwt, err := NewGuestJWT(payload.Token)
	if err != nil {
		return GuestSession{}, err
	}
	c.SetGuestJWT(jwt)
	return GuestSession{Token: jwt}, nil
}

func (c *Client) refreshGuestAfterFailure(ctx context.Context, stale *GuestJWT) (GuestSession, error) {
	c.refreshLock.Lock()
	defer c.refreshLock.Unlock()
	if stale != nil {
		if current, ok := c.GuestJWT(); ok && current.Value() != stale.Value() {
			return GuestSession{Token: current}, nil
		}
	}
	return c.startGuestSessionLocked(ctx)
}

func (c *Client) ActivationPreimage(txSig string, selectedLeagues []int) (string, error) {
	jwt, ok := c.GuestJWT()
	if !ok {
		return "", ErrMissingGuestJWT
	}
	return ActivationPreimage(txSig, selectedLeagues, jwt), nil
}

func (c *Client) ActivateSubscription(ctx context.Context, txSig string, selectedLeagues []int, walletSignatureBase64 string) (APIToken, error) {
	if strings.TrimSpace(txSig) == "" {
		return APIToken{}, newError(ErrInvalidInput, "subscription transaction signature must not be empty")
	}
	if strings.TrimSpace(walletSignatureBase64) == "" {
		return APIToken{}, newError(ErrInvalidInput, "wallet activation signature must not be empty")
	}
	var out json.RawMessage
	err := c.postJSON(ctx, "/token/activate", activationPayload{
		TxSig: txSig, WalletSignature: walletSignatureBase64, Leagues: selectedLeagues,
	}, false, &out)
	if err != nil {
		return APIToken{}, err
	}
	var token string
	if len(out) > 0 && out[0] == '"' {
		if err := json.Unmarshal(out, &token); err != nil {
			return APIToken{}, err
		}
	} else {
		var response tokenResponse
		if err := json.Unmarshal(out, &response); err != nil {
			token = string(out)
		} else {
			token = response.Token
		}
	}
	apiToken, err := NewAPIToken(token)
	if err != nil {
		return APIToken{}, err
	}
	c.SetAPIToken(apiToken)
	return apiToken, nil
}

func (c *Client) Fixtures() FixturesClient {
	return FixturesClient{client: c}
}

func (c *Client) Odds() OddsClient {
	return OddsClient{client: c}
}

func (c *Client) Scores() ScoresClient {
	return ScoresClient{client: c}
}

func (c *Client) getJSON(ctx context.Context, endpoint string, query url.Values, requireAPIToken bool, out any) error {
	return c.requestJSON(ctx, http.MethodGet, endpoint, query, nil, requireAPIToken, out)
}

func (c *Client) postJSON(ctx context.Context, endpoint string, body any, requireAPIToken bool, out any) error {
	encoded, err := json.Marshal(body)
	if err != nil {
		return err
	}
	return c.requestJSON(ctx, http.MethodPost, endpoint, nil, encoded, requireAPIToken, out)
}

func (c *Client) requestJSON(ctx context.Context, method, endpoint string, query url.Values, body []byte, requireAPIToken bool, out any) error {
	stale, _ := c.GuestJWT()
	resp, err := c.sendRequest(ctx, method, endpoint, query, body, requireAPIToken)
	if err != nil {
		return err
	}
	if resp.StatusCode == http.StatusUnauthorized {
		resp.Body.Close()
		_, err := c.refreshGuestAfterFailure(ctx, &stale)
		if err != nil {
			return err
		}
		resp, err = c.sendRequest(ctx, method, endpoint, query, body, requireAPIToken)
		if err != nil {
			return err
		}
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return statusError(resp)
	}
	return json.NewDecoder(resp.Body).Decode(out)
}

func (c *Client) sendRequest(ctx context.Context, method, endpoint string, query url.Values, body []byte, requireAPIToken bool) (*http.Response, error) {
	var reader io.Reader
	if body != nil {
		reader = bytes.NewReader(body)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.apiURL(endpoint, query), reader)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "application/json")
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	headers, err := c.AuthHeaders(requireAPIToken)
	if err != nil {
		return nil, err
	}
	headers.Apply(req.Header)
	return c.http.Do(req)
}

func (c *Client) apiURL(endpoint string, query url.Values) string {
	base, _ := url.Parse(c.config.APIBase)
	base.Path = path.Join(base.Path, strings.TrimPrefix(endpoint, "/"))
	if query != nil {
		base.RawQuery = query.Encode()
	}
	return base.String()
}

func statusError(resp *http.Response) error {
	body, _ := io.ReadAll(resp.Body)
	return &HTTPStatusError{StatusCode: resp.StatusCode, Body: body}
}

func addInt64(values url.Values, key string, value int64) {
	values.Set(key, strconv.FormatInt(value, 10))
}

func addInt32(values url.Values, key string, value int32) {
	values.Set(key, strconv.FormatInt(int64(value), 10))
}

func addUint32(values url.Values, key string, value uint32) {
	values.Set(key, strconv.FormatUint(uint64(value), 10))
}
