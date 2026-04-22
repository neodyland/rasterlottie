#![cfg(test)]

#[cfg(feature = "gif")]
use std::io::Cursor;

use super::{
    tests::{assert_fixture_supported, color_sum, load_fixture_animation, pixel_at},
    *,
};
use crate::{Animation, analyze_animation};

#[cfg(feature = "gif")]
#[derive(Debug)]
struct DecodedGifFrame {
    delay: u16,
    dispose: gif::DisposalMethod,
    raster: RasterFrame,
}

#[cfg(feature = "gif")]
fn decode_gif_rgba_frames(bytes: &[u8]) -> Vec<DecodedGifFrame> {
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(Cursor::new(bytes)).unwrap();
    let mut frames = Vec::new();

    while let Some(frame) = decoder.read_next_frame().unwrap() {
        frames.push(DecodedGifFrame {
            delay: frame.delay,
            dispose: frame.dispose,
            raster: RasterFrame {
                width: u32::from(frame.width),
                height: u32::from(frame.height),
                pixels: frame.buffer.to_vec(),
            },
        });
    }

    frames
}

#[test]
fn renderer_respects_rounded_rectangle_corners() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":0,"k":[50,50]},"s":{"a":0,"k":[40,40]},"r":{"a":0,"k":10}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0.5,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 31, 31), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 40, 40), [255, 128, 0, 255]);
}

#[test]
fn renderer_fills_a_static_ellipse() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"el","p":{"a":0,"k":[50,50]},"s":{"a":0,"k":[30,20]}},
                                    {"ty":"fl","c":{"a":0,"k":[0,0,1,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 50, 50), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&frame, 20, 20), [0, 0, 0, 0]);
}

#[test]
fn renderer_strokes_a_static_rectangle() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":0,"k":[50,50]},"s":{"a":0,"k":[40,40]},"r":{"a":0,"k":0}},
                                    {"ty":"st","c":{"a":0,"k":[1,1,1,1]},"o":{"a":0,"k":100},"w":{"a":0,"k":8},"lc":2,"lj":2,"ml":4},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 30, 50), [255, 255, 255, 255]);
    assert_eq!(pixel_at(&frame, 50, 50), [0, 0, 0, 0]);
}

#[test]
fn renderer_fills_a_static_shape_path() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"sh","ks":{"a":0,"k":{"c":true,"i":[[0,0],[0,0],[0,0]],"o":[[0,0],[0,0],[0,0]],"v":[[20,80],[50,20],[80,80]]}}},
                                    {"ty":"fl","c":{"a":0,"k":[0.5,0,1,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 50, 60), [128, 0, 255, 255]);
    assert_eq!(pixel_at(&frame, 10, 10), [0, 0, 0, 0]);
}

#[test]
fn renderer_strokes_an_open_shape_path() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"sh","ks":{"a":0,"k":{"c":false,"i":[[0,0],[0,0]],"o":[[0,0],[0,0]],"v":[[20,20],[80,80]]}}},
                                    {"ty":"st","c":{"a":0,"k":[1,1,0,1]},"o":{"a":0,"k":100},"w":{"a":0,"k":6},"lc":2,"lj":2,"ml":4},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 50, 50), [255, 255, 0, 255]);
    assert_eq!(pixel_at(&frame, 50, 20), [0, 0, 0, 0]);
}

#[test]
fn renderer_interpolates_animated_rectangle_position() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":1,"k":[{"t":0,"s":[20,50],"e":[80,50],"i":{"x":[1,1],"y":[1,1]},"o":{"x":[0,0],"y":[0,0]}},{"t":10,"s":[80,50]}]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let report = analyze_animation(&animation);
    assert!(report.is_supported(), "{report}");

    let early = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();
    let middle = Renderer::default()
        .render_frame(&animation, 5.0, RenderConfig::default())
        .unwrap();
    let late = Renderer::default()
        .render_frame(&animation, 10.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&early, 20, 50), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&middle, 50, 50), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&late, 80, 50), [255, 0, 0, 255]);
}

#[test]
fn renderer_interpolates_split_position() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "ks":{
                            "a":{"a":0,"k":[0,0]},
                            "p":{"s":1,"x":{"a":1,"k":[{"t":0,"s":[20],"e":[80],"i":{"x":1,"y":1},"o":{"x":0,"y":0}},{"t":10,"s":[80]}]},"y":{"a":0,"k":50}},
                            "s":{"a":0,"k":[100,100]},
                            "r":{"a":0,"k":0},
                            "o":{"a":0,"k":100}
                        },
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0.5,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let report = analyze_animation(&animation);
    assert!(report.is_supported(), "{report}");

    let early = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();
    let middle = Renderer::default()
        .render_frame(&animation, 5.0, RenderConfig::default())
        .unwrap();
    let late = Renderer::default()
        .render_frame(&animation, 10.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&early, 20, 50), [255, 128, 0, 255]);
    assert_eq!(pixel_at(&middle, 50, 50), [255, 128, 0, 255]);
    assert_eq!(pixel_at(&late, 80, 50), [255, 128, 0, 255]);
}

#[test]
fn renderer_interpolates_spatial_rectangle_position() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":1,"k":[{"t":0,"s":[20,80],"e":[80,80],"to":[0,-80],"ti":[0,-80],"i":{"x":1,"y":1},"o":{"x":0,"y":0}},{"t":10,"s":[80,80]}]},"s":{"a":0,"k":[16,16]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[0,0,1,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let report = analyze_animation(&animation);
    assert!(report.is_supported(), "{report}");

    let early = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();
    let middle = Renderer::default()
        .render_frame(&animation, 5.0, RenderConfig::default())
        .unwrap();
    let late = Renderer::default()
        .render_frame(&animation, 10.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&early, 20, 80), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&middle, 50, 20), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&late, 80, 80), [0, 0, 255, 255]);
}

#[test]
fn renderer_interpolates_animated_shape_paths() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":100,
                "h":100,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"sh","ks":{"a":1,"k":[
                                        {"t":0,"s":[{"c":true,"i":[[0,0],[0,0],[0,0],[0,0]],"o":[[0,0],[0,0],[0,0],[0,0]],"v":[[10,30],[30,30],[30,70],[10,70]]}],"e":[{"c":true,"i":[[0,0],[0,0],[0,0],[0,0]],"o":[[0,0],[0,0],[0,0],[0,0]],"v":[[70,30],[90,30],[90,70],[70,70]]}],"i":{"x":1,"y":1},"o":{"x":0,"y":0}},
                                        {"t":10,"s":[{"c":true,"i":[[0,0],[0,0],[0,0],[0,0]],"o":[[0,0],[0,0],[0,0],[0,0]],"v":[[70,30],[90,30],[90,70],[70,70]]}]}
                                    ]}},
                                    {"ty":"fl","c":{"a":0,"k":[0,1,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let report = analyze_animation(&animation);
    assert!(report.is_supported(), "{report}");

    let early = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();
    let middle = Renderer::default()
        .render_frame(&animation, 5.0, RenderConfig::default())
        .unwrap();
    let late = Renderer::default()
        .render_frame(&animation, 10.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&early, 20, 50), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&middle, 50, 50), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&late, 80, 50), [0, 255, 0, 255]);
}

#[cfg(feature = "gif")]
#[test]
fn renderer_encodes_a_gif_for_animated_content() {
    let animation = Animation::from_json_str(
            r#"{
                "v":"5.7.6",
                "fr":10,
                "ip":0,
                "op":10,
                "w":32,
                "h":32,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":1,"k":[{"t":0,"s":[8,16],"e":[24,16],"i":{"x":[1,1],"y":[1,1]},"o":{"x":[0,0],"y":[0,0]}},{"t":9,"s":[24,16]}]},"s":{"a":0,"k":[8,8]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

    let bytes = Renderer::default()
        .render_gif(&animation, GifRenderConfig::default())
        .unwrap();

    assert!(bytes.starts_with(b"GIF89a") || bytes.starts_with(b"GIF87a"));
    let frames = decode_gif_rgba_frames(&bytes);

    assert_eq!(frames.len(), 10);
    assert!(frames.iter().all(|frame| frame.delay == 10));
    assert!(
        frames
            .iter()
            .all(|frame| frame.dispose == gif::DisposalMethod::Background)
    );

    let first = &frames[0].raster;
    let last = &frames[frames.len() - 1].raster;
    assert_eq!(pixel_at(first, 8, 16), [255, 0, 0, 255]);
    assert_eq!(pixel_at(first, 24, 16), [0, 0, 0, 0]);
    assert_eq!(pixel_at(last, 24, 16), [255, 0, 0, 255]);
    assert_eq!(pixel_at(last, 8, 16), [0, 0, 0, 0]);
    assert_ne!(first, last);
}

#[cfg(feature = "gif")]
#[test]
fn renderer_uses_background_disposal_for_gif_frames() {
    let animation = Animation::from_json_str(
        r#"{
                "v":"5.7.6",
                "fr":10,
                "ip":0,
                "op":2,
                "w":16,
                "h":16,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":1,"k":[{"t":0,"s":[4,8],"e":[12,8],"i":{"x":[1,1],"y":[1,1]},"o":{"x":[0,0],"y":[0,0]}},{"t":1,"s":[12,8]}]},"s":{"a":0,"k":[4,4]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
    )
    .unwrap();

    let bytes = Renderer::default()
        .render_gif(&animation, GifRenderConfig::default())
        .unwrap();

    let decoder = gif::DecodeOptions::new();
    let mut decoder = decoder.read_info(Cursor::new(bytes)).unwrap();
    let mut disposals = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        disposals.push(frame.dispose);
    }

    assert!(!disposals.is_empty());
    assert!(
        disposals
            .iter()
            .all(|dispose| *dispose == gif::DisposalMethod::Background)
    );
}

#[cfg(feature = "gif")]
#[test]
fn renderer_quantizes_gif_frame_rate_to_centiseconds() {
    let animation = Animation::from_json_str(
        r#"{
                "v":"5.7.6",
                "fr":60,
                "ip":0,
                "op":60,
                "w":16,
                "h":16,
                "layers":[
                    {
                        "nm":"Shape Layer 1",
                        "ind":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":1,"k":[{"t":0,"s":[4,8],"e":[12,8],"i":{"x":[1,1],"y":[1,1]},"o":{"x":[0,0],"y":[0,0]}},{"t":59,"s":[12,8]}]},"s":{"a":0,"k":[4,4]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
    )
    .unwrap();

    let bytes = Renderer::default()
        .render_gif(
            &animation,
            GifRenderConfig {
                max_fps: 60.0,
                max_duration_seconds: 1.0,
                ..GifRenderConfig::default()
            },
        )
        .unwrap();

    let frames = decode_gif_rgba_frames(&bytes);

    assert_eq!(frames.len(), 50);
    assert!(frames.iter().all(|frame| frame.delay == 2));
    assert_eq!(pixel_at(&frames[0].raster, 4, 8), [255, 0, 0, 255]);
    assert_eq!(
        pixel_at(&frames[frames.len() - 1].raster, 12, 8),
        [255, 0, 0, 255]
    );
}

#[test]
fn fixture_linear_gradient_opacity_stops_behave_as_expected() {
    let animation = load_fixture_animation("gradient_linear_opacity_stops.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    let left = pixel_at(&frame, 25, 50);
    let middle = pixel_at(&frame, 50, 50);
    let right = pixel_at(&frame, 75, 50);

    assert!(left[0] > left[2], "left pixel should skew red: {left:?}");
    assert!(
        right[2] > right[0],
        "right pixel should skew blue: {right:?}"
    );
    assert!(
        left[3] < middle[3] && middle[3] < right[3],
        "opacity stops should increase alpha across the gradient: left={left:?} middle={middle:?} right={right:?}"
    );
}

#[test]
fn fixture_radial_gradient_highlight_offsets_the_focus() {
    let animation = load_fixture_animation("gradient_radial_highlight_angle.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    let upper = pixel_at(&frame, 50, 34);
    let lower = pixel_at(&frame, 50, 66);
    let edge = pixel_at(&frame, 78, 50);

    assert!(
        color_sum(lower) > color_sum(upper),
        "highlight angle should bias the bright area downward: upper={upper:?} lower={lower:?}"
    );
    assert!(
        color_sum(edge) < color_sum(lower),
        "edge should remain darker than the shifted highlight: edge={edge:?} lower={lower:?}"
    );
}

#[test]
fn fixture_subtract_mask_cuts_a_hole() {
    let animation = load_fixture_animation("mask_subtract_hole.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 25, 50), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&frame, 50, 50), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 75, 50), [255, 0, 0, 255]);
}

#[test]
fn fixture_track_matte_prefers_explicit_parent_over_index_fallback() {
    let animation = load_fixture_animation("track_matte_parent_redirect.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 30, 50), [0, 255, 0, 255]);
    assert_eq!(pixel_at(&frame, 70, 50), [0, 0, 0, 0]);
}

#[test]
fn fixture_layer_parenting_applies_parent_transform() {
    let animation = load_fixture_animation("layer_parenting_basic.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 20, 50), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 40, 50), [0, 255, 0, 255]);
}

#[test]
fn parent_opacity_does_not_hide_child_layers() {
    let animation = Animation::from_json_str(
        r#"{
                "v":"5.7.6",
                "fr":30,
                "ip":0,
                "op":60,
                "w":64,
                "h":64,
                "layers":[
                    {
                        "nm":"Controller",
                        "ind":1,
                        "ty":3,
                        "ks":{
                            "a":{"a":0,"k":[0,0]},
                            "p":{"a":0,"k":[16,0]},
                            "s":{"a":0,"k":[100,100]},
                            "r":{"a":0,"k":0},
                            "o":{"a":0,"k":0}
                        }
                    },
                    {
                        "nm":"Child",
                        "ind":2,
                        "parent":1,
                        "ty":4,
                        "shapes":[
                            {
                                "ty":"gr",
                                "it":[
                                    {"ty":"rc","p":{"a":0,"k":[16,32]},"s":{"a":0,"k":[16,16]},"r":{"a":0,"k":0}},
                                    {"ty":"fl","c":{"a":0,"k":[0,1,0,1]},"o":{"a":0,"k":100}},
                                    {"ty":"tr","a":{"a":0,"k":[0,0]},"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}
                                ]
                            }
                        ]
                    }
                ]
            }"#,
    )
    .unwrap();
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 16, 32), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 32, 32), [0, 255, 0, 255]);
}

#[test]
fn fixture_stroke_dash_creates_gaps() {
    let animation = load_fixture_animation("stroke_dash_basic.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 15, 50), [255, 255, 255, 255]);
    assert_eq!(pixel_at(&frame, 25, 50), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 35, 50), [255, 255, 255, 255]);
}

#[test]
fn fixture_trim_path_limits_the_visible_segment() {
    let animation = load_fixture_animation("trim_path_basic.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 15, 50), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&frame, 50, 50), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&frame, 85, 50), [0, 0, 0, 0]);
}

#[test]
fn fixture_polystar_draws_a_filled_star() {
    let animation = load_fixture_animation("polystar_basic.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 50, 50), [255, 255, 0, 255]);
    assert_eq!(pixel_at(&frame, 10, 10), [0, 0, 0, 0]);
}

#[test]
fn fixture_repeater_duplicates_the_source_geometry() {
    let animation = load_fixture_animation("repeater_basic.json");
    assert_fixture_supported(&animation);

    let frame = Renderer::default()
        .render_frame(&animation, 0.0, RenderConfig::default())
        .unwrap();

    assert_eq!(pixel_at(&frame, 20, 50), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&frame, 40, 50), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&frame, 60, 50), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&frame, 80, 50), [0, 0, 0, 0]);
}
