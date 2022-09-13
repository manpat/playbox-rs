pub mod global;
pub mod audio_test;
pub mod debug;
pub mod debug_camera;
pub mod camera;
pub mod player;
pub mod gem;

pub use global::*;
pub use audio_test::*;
pub use debug::*;
pub use debug_camera::*;
pub use camera::*;
pub use player::*;
pub use gem::*;


use toybox::prelude::*;


// TODO(pat.m): for thr love of god move this somewhere else and expand on it
pub fn load_audio_buffer(asset_path: impl AsRef<std::path::Path>) -> Result<Vec<f32>, Box<dyn Error>> {
	use symphonia::default;
	use symphonia::core::io::MediaSourceStream;
	use symphonia::core::probe::Hint;
	use symphonia::core::codecs::DecoderOptions;
	use symphonia::core::formats::FormatOptions;
	use symphonia::core::meta::MetadataOptions;
	use symphonia::core::audio::SampleBuffer;
	use symphonia::core::errors::Error;

	use std::fs::File;
	use std::ffi::OsStr;


	let asset_path = asset_path.as_ref();
	let file = File::open(asset_path)?;
	let stream = MediaSourceStream::new(Box::new(file), Default::default());

	let mut hint = Hint::new();
	if let Some(extension) = asset_path.extension().and_then(OsStr::to_str) {
		hint.with_extension(extension);
	}

	let metadata_options = MetadataOptions::default();
	let format_options = FormatOptions::default();

	let probed_format = default::get_probe().format(&hint, stream, &format_options, &metadata_options)?;
	let mut format = probed_format.format;

	let track = format.default_track()
		.expect("no supported audio tracks");


	let decoder_options = DecoderOptions::default();

	let mut decoder = default::get_codecs()
		.make(&track.codec_params, &decoder_options)
		.expect("unsupported codec");

		
	// Store the track identifier, it will be used to filter packets.
	let track_id = track.id;

    let mut sample_buf = None;
    // let mut sample_count = 0;

    let mut samples = Vec::new();

	// The decode loop.
	loop {
		// Get the next packet from the media format.
		let packet = match format.next_packet() {
			Ok(packet) => packet,
			Err(Error::ResetRequired) => {
				// The track list has been changed. Re-examine it and create a new set of decoders,
				// then restart the decode loop. This is an advanced feature and it is not
				// unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
				// for chained OGG physical streams.
				unimplemented!();
			}
			Err(Error::IoError(_)) => {
				break
			}
			Err(err) => {
				// A unrecoverable error occured, halt decoding.
				panic!("AAAAAAAA {:?}", err);
			}
		};

		// If the packet does not belong to the selected track, skip over it.
		if packet.track_id() != track_id {
			continue;
		}

		// Decode the packet into audio samples.
		match decoder.decode(&packet) {
			Ok(audio_buf) => {
				// Consume the decoded audio samples (see below).
                if sample_buf.is_none() {
                    // Get the audio buffer specification.
                    let spec = *audio_buf.spec();

                    // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                    let duration = audio_buf.capacity() as u64;

                    // Create the f32 sample buffer.
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);

                    samples.extend_from_slice(buf.samples());

                    // The samples may now be access via the `samples()` function.
                    // sample_count += buf.samples().len();
                    // println!("Decoded {} samples", buf.samples().len());
                }
			}
			Err(Error::IoError(_)) => {
				// The packet failed to decode due to an IO error, skip the packet.
				continue;
			}
			Err(Error::DecodeError(_)) => {
				// The packet failed to decode due to invalid data, skip the packet.
				continue;
			}
			Err(err) => {
				// An unrecoverable error occured, halt decoding.
				panic!("AAAAAAA {}", err);
			}
		}
	}

	Ok(samples)
}