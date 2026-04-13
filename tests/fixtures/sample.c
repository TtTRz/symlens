#include <stdio.h>
#include <math.h>

#define MAX_CHANNELS 8

// AudioFormat represents audio encoding.
typedef enum {
    FORMAT_MONO,
    FORMAT_STEREO,
    FORMAT_SURROUND
} AudioFormat;

// AudioEngine processes audio samples.
typedef struct {
    int channels;
    int sample_rate;
    AudioFormat format;
} AudioEngine;

// SampleRate is an alias for int.
typedef int SampleRate;

// normalize clamps a sample to [-1.0, 1.0].
float normalize(float sample) {
    if (sample > 1.0f) return 1.0f;
    if (sample < -1.0f) return -1.0f;
    return sample;
}

// process_block processes audio samples.
void process_block(AudioEngine* engine, float* samples, int count) {
    for (int i = 0; i < count; i++) {
        samples[i] = normalize(samples[i]);
    }
    printf("Processed %d samples\n", count);
}
