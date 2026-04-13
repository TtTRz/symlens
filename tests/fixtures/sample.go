// Package sample provides audio processing utilities.
package sample

import (
	"fmt"
	"math"
)

// MaxChannels is the maximum number of audio channels supported.
const MaxChannels = 8

// DefaultRate is the default sample rate.
var DefaultRate = 44100

// SampleRate represents a sample rate in Hz.
type SampleRate = int

// AudioFormat represents an audio encoding format.
type AudioFormat int

const (
	FormatMono     AudioFormat = iota
	FormatStereo
	FormatSurround
)

// AudioEngine processes audio samples.
// It supports multiple channels and configurable sample rates.
type AudioEngine struct {
	Channels   int
	SampleRate SampleRate
	Format     AudioFormat
}

// Processor defines the interface for audio processors.
type Processor interface {
	// Process processes a block of audio samples.
	Process(samples []float32) error
}

// NewAudioEngine creates a new AudioEngine with default settings.
func NewAudioEngine() *AudioEngine {
	return &AudioEngine{
		Channels:   2,
		SampleRate: DefaultRate,
		Format:     FormatStereo,
	}
}

// ProcessBlock processes a block of audio samples.
func (e *AudioEngine) ProcessBlock(samples []float32) {
	for i, s := range samples {
		samples[i] = Normalize(s)
	}
	fmt.Printf("Processed %d samples\n", len(samples))
}

// Normalize clamps a sample to the range [-1.0, 1.0].
func Normalize(sample float32) float32 {
	return float32(math.Max(-1.0, math.Min(1.0, float64(sample))))
}
