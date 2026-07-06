package txline

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestAuthValidationRedactionAndActivationPreimage(t *testing.T) {
	if _, err := NewGuestJWT(" "); !errors.Is(err, ErrInvalidInput) {
		t.Fatalf("empty guest JWT should be invalid: %v", err)
	}
	if _, err := NewAPIToken(""); !errors.Is(err, ErrInvalidInput) {
		t.Fatalf("empty API token should be invalid: %v", err)
	}

	jwt, err := NewGuestJWT("jwt.secret.value")
	if err != nil {
		t.Fatal(err)
	}
	token, err := NewAPIToken("api.secret.value")
	if err != nil {
		t.Fatal(err)
	}
	if got := fmt.Sprintf("%s %s", jwt, token); strings.Contains(got, "secret") {
		t.Fatalf("secret leaked in string output: %s", got)
	}
	if got := ActivationPreimage("txsig", []int{1, 2}, jwt); got != "txsig:1,2:jwt.secret.value" {
		t.Fatalf("activation preimage mismatch: %s", got)
	}
	if got := ActivationPreimage("txsig", nil, jwt); got != "txsig::jwt.secret.value" {
		t.Fatalf("empty bundle preimage mismatch: %s", got)
	}
}

func TestDevnetConfigGuardrails(t *testing.T) {
	if _, err := NewClient(DevnetConfig().WithRPCURL("https://api.mainnet-beta.solana.com")); !errors.Is(err, ErrConfig) {
		t.Fatalf("mainnet RPC should be rejected: %v", err)
	}
	cfg := DevnetConfig()
	cfg.ProgramID = "9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA"
	if _, err := NewClient(cfg); !errors.Is(err, ErrConfig) {
		t.Fatalf("mixed program config should be rejected: %v", err)
	}
}

func TestHTTPQueryConstructionAndV2StatKeys(t *testing.T) {
	var sawStatKeys bool
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer jwt" || r.Header.Get(APITokenHeader) != "api" {
			t.Fatalf("auth headers not sent safely: %v", r.Header)
		}
		switch r.URL.Path {
		case "/api/fixtures/snapshot":
			if got := r.URL.Query().Get("startEpochDay"); got != "20615" {
				t.Fatalf("startEpochDay query mismatch: %s", got)
			}
			if got := r.URL.Query().Get("competitionId"); got != "7" {
				t.Fatalf("competitionId query mismatch: %s", got)
			}
			_ = json.NewEncoder(w).Encode([]Fixture{})
		case "/api/scores/stat-validation":
			sawStatKeys = true
			if got := r.URL.Query().Get("fixtureId"); got != "17952170" {
				t.Fatalf("fixtureId query mismatch: %s", got)
			}
			if got := r.URL.Query().Get("seq"); got != "3" {
				t.Fatalf("seq query mismatch: %s", got)
			}
			if got := r.URL.Query().Get("statKeys"); got != "1001,1002" {
				t.Fatalf("statKeys query mismatch: %s", got)
			}
			_ = json.NewEncoder(w).Encode(v2ResponseForTest(2))
		default:
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
	}))
	defer server.Close()

	cfg := DevnetConfig()
	cfg.APIBase = server.URL + "/api"
	client := newUncheckedClient(cfg, server.Client())
	jwt, _ := NewGuestJWT("jwt")
	token, _ := NewAPIToken("api")
	client.SetGuestJWT(jwt)
	client.SetAPIToken(token)

	if _, err := client.Fixtures().Snapshot(context.Background(), ptrUint32(20615), ptrInt32(7)); err != nil {
		t.Fatalf("fixtures snapshot: %v", err)
	}
	validation, err := client.Scores().StatValidationV2(context.Background(), 17_952_170, 3, []uint32{1001, 1002})
	if err != nil {
		t.Fatalf("stat validation v2: %v", err)
	}
	if !sawStatKeys {
		t.Fatal("statKeys path was not exercised")
	}
	if got := validation.RequestedStatKeys(); fmt.Sprint(got) != "[1001 1002]" {
		t.Fatalf("requested keys not preserved: %v", got)
	}
}
