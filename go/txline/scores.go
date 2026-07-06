package txline

import (
	"context"
	"net/url"
	"strconv"
	"strings"
)

type ScoresClient struct {
	client *Client
}

func (s ScoresClient) Snapshot(ctx context.Context, fixtureID int64, asOf *int64) ([]Scores, error) {
	query := url.Values{}
	if asOf != nil {
		addInt64(query, "asOf", *asOf)
	}
	var out []Scores
	err := s.client.getJSON(ctx, "/scores/snapshot/"+strconv.FormatInt(fixtureID, 10), query, true, &out)
	return out, err
}

func (s ScoresClient) LiveUpdatesByFixture(ctx context.Context, fixtureID int64) ([]Scores, error) {
	var out []Scores
	err := s.client.getJSON(ctx, "/scores/updates/"+strconv.FormatInt(fixtureID, 10), nil, true, &out)
	return out, err
}

func (s ScoresClient) HistoricalUpdates(ctx context.Context, epochDay uint32, hourOfDay, interval uint8, fixtureID *int64) ([]Scores, error) {
	if err := validateHour(hourOfDay); err != nil {
		return nil, err
	}
	if err := validateInterval(interval); err != nil {
		return nil, err
	}
	query := url.Values{}
	if fixtureID != nil {
		addInt64(query, "fixtureId", *fixtureID)
	}
	endpoint := "/scores/updates/" + strconv.FormatUint(uint64(epochDay), 10) + "/" + strconv.Itoa(int(hourOfDay)) + "/" + strconv.Itoa(int(interval))
	var out []Scores
	err := s.client.getJSON(ctx, endpoint, query, true, &out)
	return out, err
}

func (s ScoresClient) HistoricalByFixture(ctx context.Context, fixtureID int64) ([]Scores, error) {
	var out []Scores
	err := s.client.getJSON(ctx, "/scores/historical/"+strconv.FormatInt(fixtureID, 10), nil, true, &out)
	return out, err
}

func (s ScoresClient) StatValidationLegacy(ctx context.Context, fixtureID int64, seq int32, statKey uint32, statKey2 *uint32) (ScoresStatValidation, error) {
	if err := EnsurePositiveSeq(seq); err != nil {
		return ScoresStatValidation{}, err
	}
	query := url.Values{}
	addInt64(query, "fixtureId", fixtureID)
	addInt32(query, "seq", seq)
	addUint32(query, "statKey", statKey)
	if statKey2 != nil {
		addUint32(query, "statKey2", *statKey2)
	}
	var out ScoresStatValidation
	err := s.client.getJSON(ctx, "/scores/stat-validation", query, true, &out)
	return out, err
}

func (s ScoresClient) StatValidationV2(ctx context.Context, fixtureID int64, seq int32, statKeys []uint32) (*ScoresStatValidationV2, error) {
	if err := EnsurePositiveSeq(seq); err != nil {
		return nil, err
	}
	if len(statKeys) == 0 {
		return nil, newError(ErrInvalidInput, "V2 stat validation requires at least one stat key")
	}
	parts := make([]string, len(statKeys))
	for i, key := range statKeys {
		parts[i] = strconv.FormatUint(uint64(key), 10)
	}
	query := url.Values{}
	addInt64(query, "fixtureId", fixtureID)
	addInt32(query, "seq", seq)
	query.Set("statKeys", strings.Join(parts, ","))
	var response ScoresStatValidationV2Response
	if err := s.client.getJSON(ctx, "/scores/stat-validation", query, true, &response); err != nil {
		return nil, err
	}
	return NewScoresStatValidationV2(statKeys, response)
}
