package txline

import (
	"context"
	"net/url"
	"strconv"
)

type FixturesClient struct {
	client *Client
}

func (f FixturesClient) Snapshot(ctx context.Context, startEpochDay *uint32, competitionID *int32) ([]Fixture, error) {
	query := url.Values{}
	if startEpochDay != nil {
		addUint32(query, "startEpochDay", *startEpochDay)
	}
	if competitionID != nil {
		addInt32(query, "competitionId", *competitionID)
	}
	var out []Fixture
	err := f.client.getJSON(ctx, "/fixtures/snapshot", query, true, &out)
	return out, err
}

func (f FixturesClient) Updates(ctx context.Context, epochDay uint32, hourOfDay uint8) ([]Fixture, error) {
	if err := validateHour(hourOfDay); err != nil {
		return nil, err
	}
	var out []Fixture
	err := f.client.getJSON(ctx, "/fixtures/updates/"+strconv.FormatUint(uint64(epochDay), 10)+"/"+strconv.Itoa(int(hourOfDay)), nil, true, &out)
	return out, err
}

func (f FixturesClient) Validation(ctx context.Context, fixtureID int64, timestamp *int64) (FixtureValidation, error) {
	query := url.Values{}
	addInt64(query, "fixtureId", fixtureID)
	if timestamp != nil {
		addInt64(query, "timestamp", *timestamp)
	}
	var out FixtureValidation
	err := f.client.getJSON(ctx, "/fixtures/validation", query, true, &out)
	return out, err
}

func (f FixturesClient) BatchValidation(ctx context.Context, epochDay uint32, hourOfDay uint8) (FixtureBatchValidation, error) {
	if err := validateHour(hourOfDay); err != nil {
		return FixtureBatchValidation{}, err
	}
	query := url.Values{}
	addUint32(query, "epochDay", epochDay)
	query.Set("hourOfDay", strconv.Itoa(int(hourOfDay)))
	var out FixtureBatchValidation
	err := f.client.getJSON(ctx, "/fixtures/batch-validation", query, true, &out)
	return out, err
}

func validateHour(hourOfDay uint8) error {
	if hourOfDay > 23 {
		return newError(ErrInvalidInput, "hour_of_day must be 0..=23")
	}
	return nil
}

func validateInterval(interval uint8) error {
	if interval > 11 {
		return newError(ErrInvalidInput, "interval must be the 0-indexed 5-minute bucket 0..=11")
	}
	return nil
}
