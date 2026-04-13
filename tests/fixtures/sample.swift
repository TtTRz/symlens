import Foundation

/// The maximum number of audio channels supported.
let maxChannels: Int = 8

/// AudioEngine processes audio samples.
/// It supports multiple channels and configurable sample rates.
public class AudioEngine {
    var channels: Int
    var sampleRate: Int

    /// Creates a new AudioEngine with default settings.
    public init() {
        self.channels = 2
        self.sampleRate = 44100
    }

    /// Processes a block of audio samples.
    public func processBlock(samples: inout [Float]) {
        for i in 0..<samples.count {
            samples[i] = normalize(samples[i])
        }
    }
}

/// AudioFormat describes the encoding of audio data.
struct AudioFormat {
    var bitDepth: Int
    var isFloat: Bool
}

/// ChannelLayout defines the arrangement of audio channels.
enum ChannelLayout {
    case mono
    case stereo
    case surround
}

/// Processor defines the interface for audio processors.
protocol Processor {
    func process(samples: [Float]) -> [Float]
}

/// Normalizes a sample to the range [-1.0, 1.0].
func normalize(_ sample: Float) -> Float {
    return max(-1.0, min(1.0, sample))
}
