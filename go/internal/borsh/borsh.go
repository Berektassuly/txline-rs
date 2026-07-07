package borsh

import (
	"encoding/binary"
	"fmt"
	"math"
)

type Encoder struct {
	buf []byte
}

func New() *Encoder {
	return &Encoder{}
}

func (e *Encoder) Bytes() []byte {
	out := make([]byte, len(e.buf))
	copy(out, e.buf)
	return out
}

func (e *Encoder) Raw(value []byte) {
	e.buf = append(e.buf, value...)
}

func (e *Encoder) Bool(value bool) {
	if value {
		e.U8(1)
		return
	}
	e.U8(0)
}

func (e *Encoder) U8(value uint8) {
	e.buf = append(e.buf, value)
}

func (e *Encoder) U16(value uint16) {
	e.buf = binary.LittleEndian.AppendUint16(e.buf, value)
}

func (e *Encoder) U32(value uint32) {
	e.buf = binary.LittleEndian.AppendUint32(e.buf, value)
}

func (e *Encoder) U64(value uint64) {
	e.buf = binary.LittleEndian.AppendUint64(e.buf, value)
}

func (e *Encoder) I16(value int16) {
	e.U16(uint16(value))
}

func (e *Encoder) I32(value int32) {
	e.U32(uint32(value))
}

func (e *Encoder) I64(value int64) {
	e.U64(uint64(value))
}

func (e *Encoder) String(value string) error {
	if len(value) > math.MaxUint32 {
		return fmt.Errorf("anchor string length exceeds u32")
	}
	e.U32(uint32(len(value)))
	e.buf = append(e.buf, value...)
	return nil
}

func (e *Encoder) Len(length int) error {
	if length < 0 || length > math.MaxUint32 {
		return fmt.Errorf("anchor vector length exceeds u32")
	}
	e.U32(uint32(length))
	return nil
}

func Option[T any](e *Encoder, value *T, encode func(*Encoder, T) error) error {
	if value == nil {
		e.U8(0)
		return nil
	}
	e.U8(1)
	return encode(e, *value)
}
