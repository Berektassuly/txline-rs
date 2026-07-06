package txline

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func TestSSEParserMultilineIDRetryAndHeartbeat(t *testing.T) {
	decoder := SSEDecoder{}
	events, err := decoder.Push([]byte("id: 7\nevent: scores\nretry: 25\ndata: {\"a\":1}\ndata: {\"b\":2}\n\n"))
	if err != nil {
		t.Fatal(err)
	}
	if len(events) != 1 {
		t.Fatalf("expected one event, got %d", len(events))
	}
	if events[0].ID != "7" || events[0].Event != "scores" || events[0].Retry != 25*time.Millisecond {
		t.Fatalf("metadata mismatch: %+v", events[0])
	}
	if events[0].Data != "{\"a\":1}\n{\"b\":2}" {
		t.Fatalf("multiline data mismatch: %q", events[0].Data)
	}
	heartbeat, ok, err := typedSSEEvent[Scores](RawSSEEvent{Event: "heartbeat", Data: `{"fixtureId":1}`})
	if err != nil || ok || heartbeat.ID != "" {
		t.Fatalf("heartbeat should be filtered: event=%+v ok=%v err=%v", heartbeat, ok, err)
	}
}

func TestSSEReconnectLastEventIDAndCancel(t *testing.T) {
	connections := 0
	sawLastID := false
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/scores/stream" {
			t.Fatalf("unexpected path: %s", r.URL.Path)
		}
		connections++
		w.Header().Set("Content-Type", "text/event-stream")
		if connections == 1 {
			writeSSE(t, w, "id: 1\nevent: heartbeat\ndata: {}\n\n")
			writeSSE(t, w, "id: 1\nevent: scores\ndata: "+scoreJSON(1)+"\n\n")
			return
		}
		if r.Header.Get("Last-Event-ID") == "1" {
			sawLastID = true
		}
		writeSSE(t, w, "id: 2\nevent: scores\ndata: "+scoreJSON(2)+"\n\n")
	}))
	defer server.Close()

	cfg := DevnetConfig()
	cfg.APIBase = server.URL + "/api"
	client := newUncheckedClient(cfg, server.Client())
	jwt, _ := NewGuestJWT("jwt")
	apiToken, _ := NewAPIToken("api")
	client.SetGuestJWT(jwt)
	client.SetAPIToken(apiToken)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	opts := DefaultStreamOptions()
	opts.InitialBackoff = 5 * time.Millisecond
	opts.MaxBackoff = 5 * time.Millisecond
	stream := client.Scores().Stream(ctx, opts)

	first := readScoreEvent(t, stream.Events())
	second := readScoreEvent(t, stream.Events())
	cancel()

	if first.Data.Seq != 1 || second.Data.Seq != 2 {
		t.Fatalf("unexpected seqs: %d %d", first.Data.Seq, second.Data.Seq)
	}
	if !sawLastID {
		t.Fatal("reconnect did not preserve Last-Event-ID")
	}
	select {
	case _, ok := <-stream.Events():
		if ok {
			t.Fatal("stream should close after cancellation")
		}
	case <-time.After(time.Second):
		t.Fatal("stream did not stop after cancellation")
	}
}

func TestSSERequiresAPIToken(t *testing.T) {
	cfg := DevnetConfig()
	client := newUncheckedClient(cfg, http.DefaultClient)
	jwt, _ := NewGuestJWT("jwt")
	client.SetGuestJWT(jwt)

	if _, err := client.sseResponse(context.Background(), "/scores/stream", nil, ""); err != ErrMissingAPIToken {
		t.Fatalf("expected ErrMissingAPIToken, got %v", err)
	}
}

func writeSSE(t *testing.T, w http.ResponseWriter, body string) {
	t.Helper()
	if _, err := w.Write([]byte(body)); err != nil {
		t.Fatal(err)
	}
	if f, ok := w.(http.Flusher); ok {
		f.Flush()
	}
}

func readScoreEvent(t *testing.T, events <-chan SSEEvent[Scores]) SSEEvent[Scores] {
	t.Helper()
	select {
	case event := <-events:
		return event
	case <-time.After(time.Second):
		t.Fatal("timed out waiting for score event")
	}
	return SSEEvent[Scores]{}
}

func scoreJSON(seq int32) string {
	payload := Scores{
		FixtureID:          17_952_170,
		GameState:          "inprogress",
		StartTime:          1,
		IsTeam:             true,
		FixtureGroupID:     1,
		CompetitionID:      2,
		CountryID:          3,
		SportID:            4,
		Participant1IsHome: true,
		Participant2ID:     20,
		Participant1ID:     10,
		Action:             "score",
		ID:                 seq,
		Ts:                 1_781_123_456_789,
		ConnectionID:       99,
		Seq:                seq,
		Stats:              map[string]int32{"1001": seq},
	}
	raw, _ := json.Marshal(payload)
	return string(raw)
}
