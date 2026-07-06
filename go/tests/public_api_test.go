package tests

import (
	"testing"

	txline "github.com/Berektassuly/txline/go/txline"
)

func TestPublicImportPath(t *testing.T) {
	if txline.DEVNET_PROGRAM_ID == "" {
		t.Fatal("empty Devnet program ID")
	}
}
