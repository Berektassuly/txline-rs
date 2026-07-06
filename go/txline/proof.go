package txline

import (
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"strings"
)

// Hash32 is a validation proof hash. API responses may encode it as base64,
// URL-safe base64, hex, or a 32-byte JSON array.
type Hash32 [32]byte

func NewHash32(bytes []byte) (Hash32, error) {
	var out Hash32
	if len(bytes) != 32 {
		return out, &Error{Kind: ErrProofDecode, Msg: fmt.Sprintf("expected 32 bytes, received %d", len(bytes))}
	}
	copy(out[:], bytes)
	return out, nil
}

func DecodeHash32(value string) (Hash32, error) {
	var zero Hash32
	trimmed := strings.TrimSpace(value)
	if trimmed == "" {
		return zero, &Error{Kind: ErrProofDecode, Msg: "hash string must not be empty"}
	}

	hexCandidate := strings.TrimPrefix(trimmed, "0x")
	if len(hexCandidate) == 64 && isHex(hexCandidate) {
		bytes, err := hex.DecodeString(hexCandidate)
		if err != nil {
			return zero, &Error{Kind: ErrProofDecode, Msg: err.Error(), Err: err}
		}
		return NewHash32(bytes)
	}

	encodings := []*base64.Encoding{
		base64.StdEncoding,
		base64.URLEncoding,
		base64.RawURLEncoding,
	}
	var lastErr error
	for _, encoding := range encodings {
		bytes, err := encoding.DecodeString(trimmed)
		if err == nil {
			return NewHash32(bytes)
		}
		lastErr = err
	}
	return zero, &Error{Kind: ErrProofDecode, Msg: "hash is not valid base64 or 32-byte hex", Err: lastErr}
}

func (h Hash32) Bytes() [32]byte {
	return [32]byte(h)
}

func (h Hash32) Slice() []byte {
	out := h.Bytes()
	return out[:]
}

func (h Hash32) String() string {
	return "0x" + hex.EncodeToString(h[:])
}

func (h Hash32) MarshalJSON() ([]byte, error) {
	return json.Marshal(h[:])
}

func (h *Hash32) UnmarshalJSON(data []byte) error {
	var text string
	if err := json.Unmarshal(data, &text); err == nil {
		decoded, err := DecodeHash32(text)
		if err != nil {
			return err
		}
		*h = decoded
		return nil
	}

	var bytes []byte
	if err := json.Unmarshal(data, &bytes); err == nil {
		decoded, err := NewHash32(bytes)
		if err != nil {
			return err
		}
		*h = decoded
		return nil
	}

	return &Error{Kind: ErrProofDecode, Msg: "expected hash as string or 32-byte array"}
}

type ProofNode struct {
	Hash           Hash32 `json:"hash"`
	IsRightSibling bool   `json:"isRightSibling"`
}

func (p ProofNode) AnchorHash() [32]byte {
	return p.Hash.Bytes()
}

func isHex(value string) bool {
	for _, ch := range value {
		if (ch < '0' || ch > '9') && (ch < 'a' || ch > 'f') && (ch < 'A' || ch > 'F') {
			return false
		}
	}
	return true
}
