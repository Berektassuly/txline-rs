package txline

import (
	"context"
	"net/url"
	"strconv"
)

type OddsClient struct {
	client *Client
}

func (o OddsClient) Snapshot(ctx context.Context, fixtureID int64, asOf *int64) ([]OddsPayload, error) {
	query := url.Values{}
	if asOf != nil {
		addInt64(query, "asOf", *asOf)
	}
	var out []OddsPayload
	err := o.client.getJSON(ctx, "/odds/snapshot/"+strconv.FormatInt(fixtureID, 10), query, true, &out)
	return out, err
}

func (o OddsClient) LiveUpdatesByFixture(ctx context.Context, fixtureID int64) ([]OddsPayload, error) {
	var out []OddsPayload
	err := o.client.getJSON(ctx, "/odds/updates/"+strconv.FormatInt(fixtureID, 10), nil, true, &out)
	return out, err
}

func (o OddsClient) HistoricalUpdates(ctx context.Context, epochDay uint32, hourOfDay, interval uint8, fixtureID *int64) ([]OddsPayload, error) {
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
	endpoint := "/odds/updates/" + strconv.FormatUint(uint64(epochDay), 10) + "/" + strconv.Itoa(int(hourOfDay)) + "/" + strconv.Itoa(int(interval))
	var out []OddsPayload
	err := o.client.getJSON(ctx, endpoint, query, true, &out)
	return out, err
}

func (o OddsClient) Validation(ctx context.Context, messageID string, ts int64) (OddsValidation, error) {
	query := url.Values{}
	query.Set("messageId", messageID)
	addInt64(query, "ts", ts)
	var out OddsValidation
	err := o.client.getJSON(ctx, "/odds/validation", query, true, &out)
	return out, err
}
