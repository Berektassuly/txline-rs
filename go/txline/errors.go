package txline

import (
	"errors"
	"fmt"
)

var (
	ErrMissingGuestJWT = errors.New("missing guest JWT")
	ErrMissingAPIToken = errors.New("missing API token")
	ErrInvalidInput    = errors.New("invalid input")
	ErrConfig          = errors.New("configuration error")
	ErrProofDecode     = errors.New("proof decode error")
	ErrValidation      = errors.New("validation payload error")
	ErrSolana          = errors.New("solana error")
)

type Error struct {
	Kind error
	Msg  string
	Err  error
}

func (e *Error) Error() string {
	if e.Msg == "" {
		return e.Kind.Error()
	}
	return e.Kind.Error() + ": " + e.Msg
}

func (e *Error) Unwrap() error {
	if e.Err != nil {
		return e.Err
	}
	return e.Kind
}

func newError(kind error, msg string) error {
	return &Error{Kind: kind, Msg: msg}
}

func wrapError(kind error, msg string, err error) error {
	return &Error{Kind: kind, Msg: msg, Err: err}
}

type HTTPStatusError struct {
	StatusCode int
	Body       []byte
}

func (e *HTTPStatusError) Error() string {
	if len(e.Body) == 0 {
		return fmt.Sprintf("HTTP %d: response body empty", e.StatusCode)
	}
	return fmt.Sprintf("HTTP %d: response body redacted (%d bytes)", e.StatusCode, len(e.Body))
}

func (e *HTTPStatusError) Status() int {
	return e.StatusCode
}

func (e *HTTPStatusError) RawBody() []byte {
	out := make([]byte, len(e.Body))
	copy(out, e.Body)
	return out
}
