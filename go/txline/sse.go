package txline

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"
)

type RawSSEEvent struct {
	ID    string
	Event string
	Data  string
	Retry time.Duration
}

type SSEEvent[T any] struct {
	ID    string `json:"id,omitempty"`
	Event string `json:"event,omitempty"`
	Data  T      `json:"data"`
}

type SSEDecoder struct {
	buffer strings.Builder
}

func (d *SSEDecoder) Push(bytes []byte) ([]RawSSEEvent, error) {
	d.buffer.Write(bytes)
	value := d.buffer.String()
	var events []RawSSEEvent
	for {
		block, rest, ok := splitSSEBlock(value)
		if !ok {
			break
		}
		if event, ok := ParseSSEBlock(block); ok {
			events = append(events, event)
		}
		value = rest
	}
	d.buffer.Reset()
	d.buffer.WriteString(value)
	return events, nil
}

func (d *SSEDecoder) Finish() (RawSSEEvent, bool) {
	value := d.buffer.String()
	d.buffer.Reset()
	if strings.TrimSpace(value) == "" {
		return RawSSEEvent{}, false
	}
	return ParseSSEBlock(value)
}

func ParseSSEBlock(block string) (RawSSEEvent, bool) {
	var event RawSSEEvent
	var data strings.Builder
	scanner := bufio.NewScanner(strings.NewReader(block))
	for scanner.Scan() {
		line := scanner.Text()
		line = strings.TrimSuffix(line, "\r")
		if line == "" || strings.HasPrefix(line, ":") {
			continue
		}
		field, value, found := strings.Cut(line, ":")
		if !found {
			value = ""
		} else {
			value = strings.TrimPrefix(value, " ")
		}
		switch field {
		case "id":
			event.ID = value
		case "event":
			event.Event = value
		case "data":
			data.WriteString(value)
			data.WriteByte('\n')
		case "retry":
			if retryMS, err := strconv.ParseInt(value, 10, 64); err == nil && retryMS >= 0 {
				event.Retry = time.Duration(retryMS) * time.Millisecond
			}
		}
	}
	dataValue := data.String()
	event.Data = strings.TrimSuffix(dataValue, "\n")
	if event.ID == "" && event.Event == "" && event.Data == "" && event.Retry == 0 {
		return RawSSEEvent{}, false
	}
	return event, true
}

type StreamOptions struct {
	FixtureID      *int64
	LastEventID    string
	InitialBackoff time.Duration
	MaxBackoff     time.Duration
}

func DefaultStreamOptions() StreamOptions {
	return StreamOptions{
		InitialBackoff: time.Second,
		MaxBackoff:     30 * time.Second,
	}
}

type Stream[T any] struct {
	events <-chan SSEEvent[T]
	errs   <-chan error
}

func (s *Stream[T]) Events() <-chan SSEEvent[T] { return s.events }
func (s *Stream[T]) Errors() <-chan error       { return s.errs }

func (o OddsClient) Stream(ctx context.Context, options StreamOptions) *Stream[OddsPayload] {
	return streamJSON[OddsPayload](ctx, o.client, "/odds/stream", options)
}

func (o OddsClient) StreamAll(ctx context.Context) *Stream[OddsPayload] {
	return o.Stream(ctx, DefaultStreamOptions())
}

func (o OddsClient) StreamFixture(ctx context.Context, fixtureID int64) *Stream[OddsPayload] {
	opts := DefaultStreamOptions()
	opts.FixtureID = &fixtureID
	return o.Stream(ctx, opts)
}

func (s ScoresClient) Stream(ctx context.Context, options StreamOptions) *Stream[Scores] {
	return streamJSON[Scores](ctx, s.client, "/scores/stream", options)
}

func (s ScoresClient) StreamAll(ctx context.Context) *Stream[Scores] {
	return s.Stream(ctx, DefaultStreamOptions())
}

func (s ScoresClient) StreamFixture(ctx context.Context, fixtureID int64) *Stream[Scores] {
	opts := DefaultStreamOptions()
	opts.FixtureID = &fixtureID
	return s.Stream(ctx, opts)
}

func streamJSON[T any](ctx context.Context, client *Client, path string, options StreamOptions) *Stream[T] {
	if options.InitialBackoff <= 0 {
		options.InitialBackoff = time.Second
	}
	if options.MaxBackoff <= 0 {
		options.MaxBackoff = 30 * time.Second
	}
	events := make(chan SSEEvent[T])
	errs := make(chan error, 1)
	go func() {
		defer close(events)
		defer close(errs)
		lastEventID := options.LastEventID
		backoff := options.InitialBackoff
		for {
			if err := ctx.Err(); err != nil {
				return
			}
			retry, err := streamOnce(ctx, client, path, options.FixtureID, &lastEventID, events)
			if err != nil && !errors.Is(err, context.Canceled) && !errors.Is(err, context.DeadlineExceeded) {
				select {
				case errs <- err:
				default:
				}
			}
			if retry > 0 {
				backoff = minDuration(retry, options.MaxBackoff)
			}
			if err := sleepContext(ctx, backoff); err != nil {
				return
			}
			backoff = minDuration(backoff*2, options.MaxBackoff)
		}
	}()
	return &Stream[T]{events: events, errs: errs}
}

func streamOnce[T any](ctx context.Context, client *Client, path string, fixtureID *int64, lastEventID *string, events chan<- SSEEvent[T]) (time.Duration, error) {
	resp, err := client.sseResponse(ctx, path, fixtureID, *lastEventID)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		stale, _ := client.GuestJWT()
		_, refreshErr := client.refreshGuestAfterFailure(ctx, &stale)
		if refreshErr != nil {
			return 0, refreshErr
		}
		resp.Body.Close()
		resp, err = client.sseResponse(ctx, path, fixtureID, *lastEventID)
		if err != nil {
			return 0, err
		}
		defer resp.Body.Close()
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
		return 0, &HTTPStatusError{StatusCode: resp.StatusCode, Body: body}
	}

	decoder := SSEDecoder{}
	buffer := make([]byte, 32*1024)
	var retry time.Duration
	for {
		n, readErr := resp.Body.Read(buffer)
		if n > 0 {
			rawEvents, err := decoder.Push(buffer[:n])
			if err != nil {
				return retry, err
			}
			for _, raw := range rawEvents {
				if raw.ID != "" {
					*lastEventID = raw.ID
				}
				if raw.Retry > 0 {
					retry = raw.Retry
				}
				typed, ok, err := typedSSEEvent[T](raw)
				if err != nil {
					return retry, err
				}
				if !ok {
					continue
				}
				select {
				case events <- typed:
				case <-ctx.Done():
					return retry, ctx.Err()
				}
			}
		}
		if readErr != nil {
			if readErr == io.EOF {
				if raw, ok := decoder.Finish(); ok {
					if raw.ID != "" {
						*lastEventID = raw.ID
					}
					if raw.Retry > 0 {
						retry = raw.Retry
					}
					typed, ok, err := typedSSEEvent[T](raw)
					if err != nil {
						return retry, err
					}
					if ok {
						select {
						case events <- typed:
						case <-ctx.Done():
							return retry, ctx.Err()
						}
					}
				}
				return retry, nil
			}
			return retry, readErr
		}
	}
}

func (c *Client) sseResponse(ctx context.Context, path string, fixtureID *int64, lastEventID string) (*http.Response, error) {
	query := url.Values{}
	if fixtureID != nil {
		query.Set("fixtureId", strconv.FormatInt(*fixtureID, 10))
	}
	reqURL, err := url.Parse(c.apiURL(path, query))
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, reqURL.String(), nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "text/event-stream")
	req.Header.Set("Cache-Control", "no-cache")
	if lastEventID != "" {
		req.Header.Set("Last-Event-ID", lastEventID)
	}
	headers, err := c.AuthHeaders(true)
	if err != nil {
		return nil, err
	}
	headers.Apply(req.Header)
	return c.http.Do(req)
}

func typedSSEEvent[T any](raw RawSSEEvent) (SSEEvent[T], bool, error) {
	var zero SSEEvent[T]
	if raw.Event == "heartbeat" || raw.Data == "" {
		return zero, false, nil
	}
	var data T
	if err := json.Unmarshal([]byte(raw.Data), &data); err != nil {
		return zero, false, err
	}
	return SSEEvent[T]{
		ID:    raw.ID,
		Event: raw.Event,
		Data:  data,
	}, true, nil
}

func splitSSEBlock(buffer string) (string, string, bool) {
	lf := strings.Index(buffer, "\n\n")
	crlf := strings.Index(buffer, "\r\n\r\n")
	idx, sepLen := -1, 0
	switch {
	case lf >= 0 && (crlf < 0 || lf < crlf):
		idx, sepLen = lf, 2
	case crlf >= 0:
		idx, sepLen = crlf, 4
	default:
		return "", "", false
	}
	return buffer[:idx], buffer[idx+sepLen:], true
}

func sleepContext(ctx context.Context, d time.Duration) error {
	timer := time.NewTimer(d)
	defer timer.Stop()
	select {
	case <-timer.C:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func minDuration(a, b time.Duration) time.Duration {
	if a < b {
		return a
	}
	return b
}
