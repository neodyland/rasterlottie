#[cfg(feature = "gif")]
use gif::{DisposalMethod, Encoder as GifEncoder, Repeat as GifRepeat};
use tiny_skia::{BlendMode, Pixmap, PixmapPaint, Transform as PixmapTransform};

#[cfg(feature = "gif")]
use super::gif_encode::encode_rgba_frame;
#[cfg(feature = "gif")]
pub use super::renderer_types::GifRenderConfig;
pub(super) use super::renderer_types::{
    BrushStyle, FillStyle, FrameRenderCache, GradientKind, GradientStopStyle, GradientStyle,
    LayerHierarchyCache, LayerRenderContext, LayerSliceCache, PreparedResources, RenderTransform,
    RepeaterStyle, ShapeCaches, ShapeGroupPlan, ShapePlanCache, ShapeRenderState,
    ShapeRenderableItem, ShapeRenderableKind, ShapeStyles, StaticPathCache, StrokeStyle, TrimStyle,
};
pub use super::renderer_types::{
    ImageAssetResolver, PreparedAnimation, RasterFrame, RenderConfig, Renderer, Rgba8,
};
use super::{
    assets::resolve_image_assets,
    composition::{
        LayerSurface, apply_alpha_mask, apply_track_matte, build_layer_mask, composite_pixmap,
        crop_layer_surface, find_track_matte_source_index, new_pixmap, resolve_output_canvas_size,
    },
    drawing::render_shape_items,
    layer_effects::apply_supported_layer_effects,
    sample_cache::TimelineSampleCache,
    scene::{
        frame_in_range, layer_is_visible, lookup_image_asset, lookup_precomp_asset,
        map_layer_frame, resolve_layer_transform_chain,
    },
};
use crate::{
    Animation, Layer, LayerType, RasterlottieError, SupportProfile, SupportReport,
    analyze_animation_with_profile, expression::resolve_supported_expressions,
};

impl Renderer {
    /// Creates a renderer that uses the provided support profile.
    #[must_use]
    pub const fn new(profile: SupportProfile) -> Self {
        Self { profile }
    }

    /// Creates a renderer configured with the default target-corpus support profile.
    #[must_use]
    pub const fn target_corpus() -> Self {
        Self::new(SupportProfile::target_corpus())
    }

    /// Analyzes an animation with this renderer's support profile.
    #[must_use]
    pub fn analyze(&self, animation: &Animation) -> SupportReport {
        analyze_animation_with_profile(animation, self.profile)
    }

    /// Preprocesses an animation for repeated rendering.
    ///
    /// # Errors
    ///
    /// Returns an error when the animation uses unsupported features or image
    /// asset preparation fails.
    pub fn prepare(&self, animation: &Animation) -> Result<PreparedAnimation, RasterlottieError> {
        let resolved = resolve_supported_expressions(animation);
        let report = analyze_animation_with_profile(&resolved, self.profile);
        if !report.is_supported() {
            return Err(RasterlottieError::UnsupportedFeatures { report });
        }

        let image_assets = resolve_image_assets(&resolved, None)?;
        let layer_hierarchy_cache = LayerHierarchyCache::from_animation(&resolved);
        let shape_plan_cache = ShapePlanCache::from_animation(&resolved);
        Ok(PreparedAnimation {
            renderer: *self,
            layer_hierarchy_cache,
            animation: resolved,
            image_assets,
            shape_plan_cache,
            static_path_cache: StaticPathCache::default(),
            timeline_sample_cache: TimelineSampleCache::default(),
        })
    }

    /// Preprocesses an animation while resolving external image assets through `resolver`.
    ///
    /// # Errors
    ///
    /// Returns an error when the animation uses unsupported features or the
    /// resolver cannot provide valid image bytes.
    pub fn prepare_with_resolver<R: ImageAssetResolver>(
        &self,
        animation: &Animation,
        resolver: &R,
    ) -> Result<PreparedAnimation, RasterlottieError> {
        let prepared_animation = resolve_supported_expressions(animation);
        let report = analyze_animation_with_profile(
            &prepared_animation,
            self.profile.with_external_image_assets(true),
        );
        if !report.is_supported() {
            return Err(RasterlottieError::UnsupportedFeatures { report });
        }

        let image_assets = resolve_image_assets(&prepared_animation, Some(resolver))?;
        let layer_hierarchy_cache = LayerHierarchyCache::from_animation(&prepared_animation);
        let shape_plan_cache = ShapePlanCache::from_animation(&prepared_animation);
        Ok(PreparedAnimation {
            renderer: *self,
            layer_hierarchy_cache,
            animation: prepared_animation,
            image_assets,
            shape_plan_cache,
            static_path_cache: StaticPathCache::default(),
            timeline_sample_cache: TimelineSampleCache::default(),
        })
    }

    /// Renders a single frame without retaining preprocessing state.
    ///
    /// # Errors
    ///
    /// Returns an error when preprocessing fails or the frame cannot be rasterized.
    pub fn render_frame(
        &self,
        animation: &Animation,
        frame: f32,
        config: RenderConfig,
    ) -> Result<RasterFrame, RasterlottieError> {
        self.prepare(animation)?.render_frame(frame, config)
    }

    /// Renders a single frame while allowing external image asset resolution.
    ///
    /// # Errors
    ///
    /// Returns an error when preprocessing fails, asset resolution fails, or the
    /// frame cannot be rasterized.
    pub fn render_frame_with_resolver<R: ImageAssetResolver>(
        &self,
        animation: &Animation,
        frame: f32,
        config: RenderConfig,
        resolver: &R,
    ) -> Result<RasterFrame, RasterlottieError> {
        self.prepare_with_resolver(animation, resolver)?
            .render_frame(frame, config)
    }

    fn render_frame_with_assets(
        &self,
        animation: &Animation,
        frame: f32,
        config: RenderConfig,
        resources: PreparedResources<'_>,
    ) -> Result<RasterFrame, RasterlottieError> {
        let (width, height) = resolve_output_canvas_size(animation, config)?;
        let mut pixmap = new_pixmap(width, height)?;
        self.render_frame_with_assets_into(animation, frame, config, resources, &mut pixmap)?;

        Ok(RasterFrame {
            width: pixmap.width(),
            height: pixmap.height(),
            pixels: pixmap.data().to_vec(),
        })
    }

    fn render_frame_with_assets_into(
        &self,
        animation: &Animation,
        frame: f32,
        config: RenderConfig,
        resources: PreparedResources<'_>,
        pixmap: &mut Pixmap,
    ) -> Result<(), RasterlottieError> {
        span_enter!(
            tracing::Level::TRACE,
            "render_frame_with_assets_into",
            frame = frame,
            width = pixmap.width(),
            height = pixmap.height()
        );
        let (expected_width, expected_height) = resolve_output_canvas_size(animation, config)?;
        if pixmap.width() != expected_width || pixmap.height() != expected_height {
            return Err(RasterlottieError::InvalidCanvasSize {
                width: pixmap.width(),
                height: pixmap.height(),
            });
        }

        pixmap.fill(config.background.into());

        if !frame_in_range(frame, Some(animation.in_point), Some(animation.out_point)) {
            return Ok(());
        }

        let frame_cache = FrameRenderCache::default();
        let context = LayerRenderContext {
            animation,
            layers: &animation.layers,
            image_assets: resources.image_assets,
            static_path_cache: resources.shape_caches.static_paths,
            layer_hierarchy_cache: resources.layer_hierarchy_cache,
            shape_plan_cache: resources.shape_caches.plans,
            timeline_sample_cache: resources.shape_caches.timeline_samples,
            frame_cache: &frame_cache,
            canvas_width: pixmap.width(),
            canvas_height: pixmap.height(),
        };

        self.render_layer_stack(
            &context,
            frame,
            pixmap,
            RenderTransform {
                matrix: PixmapTransform::identity().pre_scale(
                    pixmap.width() as f32 / animation.width.max(1) as f32,
                    pixmap.height() as f32 / animation.height.max(1) as f32,
                ),
                opacity: 1.0,
            },
        )
    }

    #[cfg(feature = "gif")]
    /// Renders an animation as a GIF without retaining preprocessing state.
    ///
    /// # Errors
    ///
    /// Returns an error when preprocessing fails or GIF encoding cannot be completed.
    pub fn render_gif(
        &self,
        animation: &Animation,
        config: GifRenderConfig,
    ) -> Result<Vec<u8>, RasterlottieError> {
        self.prepare(animation)?.render_gif(config)
    }

    #[cfg(feature = "gif")]
    /// Renders an animation as a GIF while allowing external image asset resolution.
    ///
    /// # Errors
    ///
    /// Returns an error when preprocessing fails, asset resolution fails, or
    /// GIF encoding cannot be completed.
    pub fn render_gif_with_resolver<R: ImageAssetResolver>(
        &self,
        animation: &Animation,
        config: GifRenderConfig,
        resolver: &R,
    ) -> Result<Vec<u8>, RasterlottieError> {
        self.prepare_with_resolver(animation, resolver)?
            .render_gif(config)
    }

    #[cfg(feature = "gif")]
    fn render_gif_with_assets(
        &self,
        animation: &Animation,
        config: GifRenderConfig,
        resources: PreparedResources<'_>,
    ) -> Result<Vec<u8>, RasterlottieError> {
        span_enter!(
            tracing::Level::TRACE,
            "render_gif_with_assets",
            width = animation.width,
            height = animation.height
        );
        let (output_width, output_height) = resolve_output_canvas_size(animation, config.render)?;
        let width =
            u16::try_from(output_width).map_err(|_| RasterlottieError::InvalidCanvasSize {
                width: output_width,
                height: output_height,
            })?;
        let height =
            u16::try_from(output_height).map_err(|_| RasterlottieError::InvalidCanvasSize {
                width: output_width,
                height: output_height,
            })?;

        let source_fps = animation.frame_rate.max(1.0);
        let requested_fps = source_fps.min(config.max_fps.max(1.0));
        let start_frame = animation.in_point.floor();
        let end_frame = animation.out_point.ceil().max(start_frame + 1.0);
        let output_duration_seconds =
            ((end_frame - start_frame) / source_fps).min(config.max_duration_seconds.max(0.1));
        let max_output_frames = (requested_fps * output_duration_seconds).floor().max(1.0) as usize;
        let source_frame_step = source_fps / requested_fps;

        let mut bytes = Vec::new();
        let mut encoder = {
            span_enter!(
                tracing::Level::TRACE,
                "gif_encoder_init",
                width = width,
                height = height,
                requested_fps = requested_fps,
                output_duration_seconds = output_duration_seconds
            );
            GifEncoder::new(&mut bytes, width, height, &[])?
        };
        encoder.set_repeat(GifRepeat::Infinite)?;
        let mut scratch = new_pixmap(output_width, output_height)?;
        let mut previous_deadline_centiseconds = 0u32;

        for rendered in 0..max_output_frames {
            let source_frame = (rendered as f32).mul_add(source_frame_step, start_frame);
            if source_frame >= end_frame {
                break;
            }
            {
                span_enter!(
                    tracing::Level::TRACE,
                    "render_gif_frame",
                    frame_index = rendered,
                    source_frame = source_frame
                );
                self.render_frame_with_assets_into(
                    animation,
                    source_frame,
                    config.render,
                    resources,
                    &mut scratch,
                )?;
            }
            let mut gif_frame = {
                span_enter!(
                    tracing::Level::TRACE,
                    "encode_gif_frame",
                    frame_index = rendered,
                    quantizer_speed = config.color_quantizer_speed
                );
                encode_rgba_frame(
                    width,
                    height,
                    scratch.data_mut(),
                    config.color_quantizer_speed,
                )
            };
            let next_deadline_centiseconds =
                ((((rendered + 1) as f64) * 100.0) / f64::from(requested_fps)).round() as u32;
            let next_deadline_centiseconds =
                next_deadline_centiseconds.max(previous_deadline_centiseconds + 1);
            let frame_delay =
                u16::try_from(next_deadline_centiseconds - previous_deadline_centiseconds)
                    .unwrap_or(u16::MAX);
            previous_deadline_centiseconds = next_deadline_centiseconds;
            gif_frame.delay = frame_delay;
            gif_frame.dispose = DisposalMethod::Background;
            {
                span_enter!(
                    tracing::Level::TRACE,
                    "write_gif_frame",
                    frame_index = rendered
                );
                encoder.write_frame(&gif_frame)?;
            }
        }

        drop(encoder);
        Ok(bytes)
    }

    fn render_layer_stack(
        &self,
        context: &LayerRenderContext<'_>,
        frame: f32,
        pixmap: &mut Pixmap,
        inherited_transform: RenderTransform,
    ) -> Result<(), RasterlottieError> {
        span_enter!(
            tracing::Level::TRACE,
            "render_layer_stack",
            frame = frame,
            layer_count = context.layers.len()
        );
        let mut consumed_mattes = vec![false; context.layers.len()];

        for index in (0..context.layers.len()).rev() {
            if consumed_mattes[index] {
                continue;
            }

            let layer = &context.layers[index];
            if layer.is_matte_source_layer() || !layer_is_visible(layer, frame) {
                continue;
            }

            if layer.track_matte_mode().is_none()
                && layer.masks.is_empty()
                && layer.effects.is_empty()
            {
                self.render_layer_content(context, frame, layer, inherited_transform, pixmap)?;
                continue;
            }

            let Some(mut surface) =
                self.render_layer_surface(context, frame, layer, inherited_transform, false)?
            else {
                continue;
            };

            if let Some(matte_mode) = layer.track_matte_mode()
                && let Some(matte_index) =
                    find_track_matte_source_index(context, context.layers, index, layer)
            {
                let matte_surface = self.render_layer_surface(
                    context,
                    frame,
                    &context.layers[matte_index],
                    inherited_transform,
                    true,
                )?;
                let Some(next_surface) =
                    apply_track_matte(surface, matte_surface.as_ref(), matte_mode)?
                else {
                    consumed_mattes[matte_index] = true;
                    continue;
                };
                surface = next_surface;
                consumed_mattes[matte_index] = true;
            }

            composite_pixmap(
                pixmap,
                &surface.pixmap,
                surface.origin_x,
                surface.origin_y,
                BlendMode::SourceOver,
            );
        }

        Ok(())
    }

    fn render_layer_content(
        &self,
        context: &LayerRenderContext<'_>,
        frame: f32,
        layer: &Layer,
        inherited_transform: RenderTransform,
        pixmap: &mut Pixmap,
    ) -> Result<(), RasterlottieError> {
        span_enter!(
            tracing::Level::TRACE,
            "render_layer_content",
            frame = frame,
            layer = layer.name.as_str(),
            layer_type = layer.layer_type.name()
        );
        let transform =
            inherited_transform.concat(resolve_layer_transform_chain(context, layer, frame));

        match layer.layer_type {
            LayerType::SHAPE => {
                let styles = ShapeStyles::default();
                render_shape_items(
                    &layer.shapes,
                    frame,
                    pixmap,
                    transform,
                    ShapeRenderState {
                        styles: &styles,
                        trim: None,
                        static_path_cache: context.static_path_cache,
                        shape_plan_cache: context.shape_plan_cache,
                        timeline_sample_cache: context.timeline_sample_cache,
                    },
                )?;
            }
            LayerType::PRECOMP => {
                let child_frame = map_layer_frame(context.animation, frame, layer);
                if let Some(asset) = lookup_precomp_asset(context.animation, layer) {
                    let child_context = LayerRenderContext {
                        animation: context.animation,
                        layers: &asset.layers,
                        image_assets: context.image_assets,
                        static_path_cache: context.static_path_cache,
                        layer_hierarchy_cache: context.layer_hierarchy_cache,
                        shape_plan_cache: context.shape_plan_cache,
                        timeline_sample_cache: context.timeline_sample_cache,
                        frame_cache: context.frame_cache,
                        canvas_width: context.canvas_width,
                        canvas_height: context.canvas_height,
                    };
                    self.render_layer_stack(&child_context, child_frame, pixmap, transform)?;
                }
            }
            LayerType::IMAGE => {
                if let Some(image) =
                    lookup_image_asset(context.image_assets, context.animation, layer)?
                {
                    let paint = PixmapPaint {
                        opacity: transform.opacity.clamp(0.0, 1.0),
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..PixmapPaint::default()
                    };
                    pixmap.draw_pixmap(
                        0,
                        0,
                        image.as_ref().as_ref(),
                        &paint,
                        transform.matrix,
                        None,
                    );
                }
            }
            LayerType::TEXT => {
                Self::render_text_layer(
                    context.animation,
                    layer,
                    frame,
                    pixmap,
                    transform,
                    ShapeCaches {
                        static_paths: context.static_path_cache,
                        plans: context.shape_plan_cache,
                        timeline_samples: context.timeline_sample_cache,
                    },
                )?;
            }
            _ => {}
        }

        Ok(())
    }

    fn render_layer_surface(
        &self,
        context: &LayerRenderContext<'_>,
        frame: f32,
        layer: &Layer,
        inherited_transform: RenderTransform,
        force_visible: bool,
    ) -> Result<Option<LayerSurface>, RasterlottieError> {
        span_enter!(
            tracing::Level::TRACE,
            "render_layer_surface",
            frame = frame,
            layer = layer.name.as_str(),
            force_visible = force_visible
        );
        if !frame_in_range(frame, layer.in_point, layer.out_point) {
            return Ok(None);
        }

        if !force_visible && layer.hidden {
            return Ok(None);
        }

        let mut pixmap = new_pixmap(context.canvas_width, context.canvas_height)?;
        self.render_layer_content(context, frame, layer, inherited_transform, &mut pixmap)?;

        apply_supported_layer_effects(&mut pixmap, layer, frame);

        if !layer.masks.is_empty() {
            let transform =
                inherited_transform.concat(resolve_layer_transform_chain(context, layer, frame));
            let mask = build_layer_mask(context, layer, frame, transform)?;
            pixmap = apply_alpha_mask(pixmap, &mask);
        }
        crop_layer_surface(&pixmap)
    }
}

impl PreparedAnimation {
    /// Returns the preprocessed animation model.
    #[must_use]
    pub const fn animation(&self) -> &Animation {
        &self.animation
    }

    /// Renders one frame while reusing the stored preprocessing results.
    ///
    /// # Errors
    ///
    /// Returns an error when the frame cannot be rasterized with the prepared assets.
    pub fn render_frame(
        &self,
        frame: f32,
        config: RenderConfig,
    ) -> Result<RasterFrame, RasterlottieError> {
        self.renderer.render_frame_with_assets(
            &self.animation,
            frame,
            config,
            PreparedResources {
                image_assets: &self.image_assets,
                shape_caches: ShapeCaches {
                    static_paths: Some(&self.static_path_cache),
                    plans: Some(&self.shape_plan_cache),
                    timeline_samples: Some(&self.timeline_sample_cache),
                },
                layer_hierarchy_cache: Some(&self.layer_hierarchy_cache),
            },
        )
    }

    #[cfg(feature = "gif")]
    /// Renders the prepared animation as a GIF.
    ///
    /// # Errors
    ///
    /// Returns an error when the prepared animation cannot be encoded as a GIF.
    pub fn render_gif(&self, config: GifRenderConfig) -> Result<Vec<u8>, RasterlottieError> {
        self.renderer.render_gif_with_assets(
            &self.animation,
            config,
            PreparedResources {
                image_assets: &self.image_assets,
                shape_caches: ShapeCaches {
                    static_paths: Some(&self.static_path_cache),
                    plans: Some(&self.shape_plan_cache),
                    timeline_samples: Some(&self.timeline_sample_cache),
                },
                layer_hierarchy_cache: Some(&self.layer_hierarchy_cache),
            },
        )
    }

    /// Allocates a scratch pixmap that matches the animation's native canvas size.
    ///
    /// # Errors
    ///
    /// Returns an error when the native canvas size is invalid.
    pub fn new_scratch_pixmap(&self) -> Result<Pixmap, RasterlottieError> {
        new_pixmap(self.animation.width, self.animation.height)
    }

    /// Allocates a scratch pixmap sized for `config`.
    ///
    /// # Errors
    ///
    /// Returns an error when `config` produces an invalid output canvas size.
    pub fn new_scratch_pixmap_for_config(
        &self,
        config: RenderConfig,
    ) -> Result<Pixmap, RasterlottieError> {
        let (width, height) = resolve_output_canvas_size(&self.animation, config)?;
        new_pixmap(width, height)
    }

    /// Renders a frame directly into an existing pixmap.
    ///
    /// # Errors
    ///
    /// Returns an error when the pixmap size does not match `config` or the
    /// frame cannot be rasterized.
    pub fn render_frame_into_pixmap(
        &self,
        frame: f32,
        config: RenderConfig,
        pixmap: &mut Pixmap,
    ) -> Result<(), RasterlottieError> {
        self.renderer.render_frame_with_assets_into(
            &self.animation,
            frame,
            config,
            PreparedResources {
                image_assets: &self.image_assets,
                shape_caches: ShapeCaches {
                    static_paths: Some(&self.static_path_cache),
                    plans: Some(&self.shape_plan_cache),
                    timeline_samples: Some(&self.timeline_sample_cache),
                },
                layer_hierarchy_cache: Some(&self.layer_hierarchy_cache),
            },
            pixmap,
        )
    }
}

impl RenderTransform {
    pub(super) fn concat(self, other: Self) -> Self {
        Self {
            matrix: self.matrix.pre_concat(other.matrix),
            opacity: self.opacity * other.opacity,
        }
    }
}
