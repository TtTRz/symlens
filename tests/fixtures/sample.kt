package sample

import kotlin.math.max
import kotlin.math.min

const val MAX_CHANNELS = 8
var defaultRate = 44100

/** AudioFormat represents audio encoding. */
enum class AudioFormat {
    MONO,
    STEREO,
    SURROUND
}

/** Processor defines an interface for audio processors. */
interface Processor {
    fun process(samples: FloatArray): FloatArray
}

/** AudioEngine processes audio samples. */
class AudioEngine(
    val channels: Int = 2,
    val sampleRate: Int = 44100
) {
    /** Processes a block of audio samples. */
    fun processBlock(samples: FloatArray): FloatArray {
        return FloatArray(samples.size) { i ->
            normalize(samples[i])
        }
    }

    private fun normalize(sample: Float): Float {
        return max(-1.0f, min(1.0f, sample))
    }
}

/** Creates a new AudioEngine with default settings. */
fun createEngine(): AudioEngine {
    return AudioEngine()
}

object AudioManager {
    fun getDefault(): AudioEngine = AudioEngine()
}
