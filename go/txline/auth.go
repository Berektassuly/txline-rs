package txline

import (
	"fmt"
	"net/http"
	"strconv"
	"strings"
)

const APITokenHeader = "X-Api-Token"

type GuestJWT struct {
	value string
}

func NewGuestJWT(value string) (GuestJWT, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return GuestJWT{}, newError(ErrInvalidInput, "guest JWT must not be empty")
	}
	return GuestJWT{value: value}, nil
}

func (g GuestJWT) Value() string {
	return g.value
}

func (g GuestJWT) String() string {
	return "GuestJWT(<redacted>)"
}

func (g GuestJWT) GoString() string {
	return g.String()
}

type APIToken struct {
	value string
}

func NewAPIToken(value string) (APIToken, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return APIToken{}, newError(ErrInvalidInput, "API token must not be empty")
	}
	return APIToken{value: value}, nil
}

func (a APIToken) Value() string {
	return a.value
}

func (a APIToken) String() string {
	return "APIToken(<redacted>)"
}

func (a APIToken) GoString() string {
	return a.String()
}

type AuthHeaders struct {
	GuestJWT GuestJWT
	APIToken *APIToken
}

func (h AuthHeaders) Apply(headers http.Header) {
	headers.Set("Authorization", "Bearer "+h.GuestJWT.Value())
	if h.APIToken != nil {
		headers.Set(APITokenHeader, h.APIToken.Value())
	}
}

func (h AuthHeaders) String() string {
	if h.APIToken != nil {
		return "AuthHeaders{Authorization:<redacted>, X-Api-Token:<redacted>}"
	}
	return "AuthHeaders{Authorization:<redacted>}"
}

func (h AuthHeaders) GoString() string {
	return h.String()
}

func ActivationPreimage(txSig string, selectedLeagues []int, jwt GuestJWT) string {
	parts := make([]string, len(selectedLeagues))
	for i, league := range selectedLeagues {
		parts[i] = strconv.Itoa(league)
	}
	return fmt.Sprintf("%s:%s:%s", txSig, strings.Join(parts, ","), jwt.Value())
}

type GuestSession struct {
	Token GuestJWT `json:"-"`
}

type tokenResponse struct {
	Token string `json:"token"`
}

type activationPayload struct {
	TxSig           string `json:"txSig"`
	WalletSignature string `json:"walletSignature"`
	Leagues         []int  `json:"leagues"`
}
