use std::borrow::Cow;

use gif::Frame as GifFrame;
use rustc_hash::FxHashMap;

pub(super) fn encode_rgba_frame(
    width: u16,
    height: u16,
    rgba: &mut [u8],
    quantizer_speed: i32,
) -> GifFrame<'static> {
    span_enter!(tracing::Level::TRACE, "gif_exact_palette");
    let transparent = canonicalize_transparent_pixels(rgba);
    let mut encounter_colors = Vec::with_capacity(256);
    let mut palette_lookup = FxHashMap::default();
    palette_lookup.reserve(256);
    let mut buffer = Vec::with_capacity(rgba.len() / 4);

    for pixel in rgba.chunks_exact(4) {
        let color = packed_rgba(pixel);
        let palette_index = if let Some(index) = palette_lookup.get(&color).copied() {
            index
        } else {
            if encounter_colors.len() == 256 {
                span_enter!(
                    tracing::Level::TRACE,
                    "gif_quantizer_fallback",
                    quantizer_speed = quantizer_speed
                );
                return GifFrame::from_rgba_speed(width, height, rgba, quantizer_speed);
            }

            encounter_colors.push(color);
            let index = (encounter_colors.len() - 1) as u8;
            palette_lookup.insert(color, index);
            index
        };
        buffer.push(palette_index);
    }

    trace!(unique_colors = encounter_colors.len(), "gif exact palette");
    let mut sorted_colors = encounter_colors.clone();
    sorted_colors.sort_unstable();
    let mut sorted_lookup = FxHashMap::default();
    sorted_lookup.reserve(sorted_colors.len());

    let mut remap = [0u8; 256];
    for (sorted_index, color) in sorted_colors.iter().copied().enumerate() {
        sorted_lookup.insert(color, sorted_index as u8);
    }
    for (encounter_index, color) in encounter_colors.iter().copied().enumerate() {
        let sorted_index = sorted_lookup.get(&color).copied().unwrap_or(0);
        remap[encounter_index] = sorted_index;
    }

    for index in &mut buffer {
        *index = remap[*index as usize];
    }

    let mut palette = Vec::with_capacity(sorted_colors.len() * 3);
    for color in &sorted_colors {
        let [r, g, b, _a] = color.to_be_bytes();
        palette.extend([r, g, b]);
    }

    let transparent_index = transparent.and_then(|color| {
        encounter_colors
            .iter()
            .position(|candidate| *candidate == color)
            .map(|encounter_index| remap[encounter_index])
    });

    GifFrame {
        width,
        height,
        buffer: Cow::Owned(buffer),
        palette: Some(palette),
        transparent: transparent_index,
        ..GifFrame::default()
    }
}

fn canonicalize_transparent_pixels(rgba: &mut [u8]) -> Option<u32> {
    let mut transparent: Option<u32> = None;

    for pixel in rgba.chunks_exact_mut(4) {
        if pixel[3] != 0 {
            pixel[3] = u8::MAX;
            continue;
        }

        if let Some(color) = transparent {
            pixel.copy_from_slice(&color.to_be_bytes());
        } else {
            transparent = Some(packed_rgba(pixel));
        }
    }

    transparent
}

fn packed_rgba(pixel: &[u8]) -> u32 {
    u32::from_be_bytes([pixel[0], pixel[1], pixel[2], pixel[3]])
}

#[cfg(test)]
mod tests {
    use gif::Frame as GifFrame;

    use super::encode_rgba_frame;

    #[test]
    fn exact_palette_encoding_matches_gif_crate_for_small_palettes() {
        let mut pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 255, 255, 10, 20, 30, 0, //
        ];
        let mut expected_pixels = pixels.clone();

        let encoded = encode_rgba_frame(2, 2, &mut pixels, 10);
        let expected = GifFrame::from_rgba_speed(2, 2, &mut expected_pixels, 10);

        assert_eq!(encoded.width, expected.width);
        assert_eq!(encoded.height, expected.height);
        assert_eq!(encoded.buffer, expected.buffer);
        assert_eq!(encoded.palette, expected.palette);
        assert_eq!(encoded.transparent, expected.transparent);
    }

    #[test]
    fn encoding_falls_back_to_gif_crate_for_large_palettes() {
        let mut pixels = Vec::with_capacity(17 * 16 * 4);
        for index in 0..272u16 {
            pixels.extend([
                (index & 0xFF) as u8,
                ((index * 3) & 0xFF) as u8,
                ((index * 5) & 0xFF) as u8,
                255,
            ]);
        }
        let mut expected_pixels = pixels.clone();

        let encoded = encode_rgba_frame(17, 16, &mut pixels, 7);
        let expected = GifFrame::from_rgba_speed(17, 16, &mut expected_pixels, 7);

        assert_eq!(encoded.buffer, expected.buffer);
        assert_eq!(encoded.palette, expected.palette);
        assert_eq!(encoded.transparent, expected.transparent);
    }
}
