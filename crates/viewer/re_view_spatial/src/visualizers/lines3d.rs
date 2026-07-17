use re_log_types::Instance;
use re_renderer::{renderer::LineStripFlags, PickingLayerInstanceId};
use re_types::{
    archetypes::LineStrips3D,
    components::{
        ClassId, Color, Colormap, DrawOrder, LineStrip3D, Radius, Scalar, ShowLabels, Text,
        ValueRange,
    },
    ArrowString, Component as _,
};
use re_view::{process_annotation_slices, process_color_slice};
use re_viewer_context::{
    auto_color_for_entity_path, IdentifiedViewSystem, MaybeVisualizableEntities, QueryContext,
    TypedComponentFallbackProvider, ViewContext, ViewContextCollection, ViewQuery,
    ViewSystemExecutionError, VisualizableEntities, VisualizableFilterContext, VisualizerQueryInfo,
    VisualizerSystem,
};

use crate::{
    contexts::SpatialSceneEntityContext,
    view_kind::SpatialViewKind,
    visualizers::utilities::{process_labels_3d, LabeledBatch},
};

use super::{filter_visualizable_3d_entities, process_radius_slice, SpatialViewVisualizerData};

// ---

const ALWAYS_ON_TOP_DRAW_ORDER: f32 = 10_000.0;

fn is_always_on_top_draw_order(draw_order: f32) -> bool {
    draw_order >= ALWAYS_ON_TOP_DRAW_ORDER
}

pub struct Lines3DVisualizer {
    pub data: SpatialViewVisualizerData,
}

impl Default for Lines3DVisualizer {
    fn default() -> Self {
        Self {
            data: SpatialViewVisualizerData::new(Some(SpatialViewKind::ThreeD)),
        }
    }
}

// NOTE: Do not put profile scopes in these methods. They are called for all entities and all
// timestamps within a time range -- it's _a lot_.
impl Lines3DVisualizer {
    fn process_data<'a>(
        &mut self,
        ctx: &QueryContext<'_>,
        line_builder: &mut re_renderer::LineDrawableBuilder<'_>,
        query: &ViewQuery<'_>,
        ent_context: &SpatialSceneEntityContext<'_>,
        data: impl Iterator<Item = Lines3DComponentData<'a>>,
    ) {
        let entity_path = ctx.target_entity_path;
        let draw_order = ctx
            .recording()
            .latest_at(ctx.query, entity_path, [DrawOrder::name()])
            .component_instance::<DrawOrder>(0)
            .unwrap_or_default()
            .0
             .0;

        for data in data {
            let num_instances = data.strips.len();
            if num_instances == 0 {
                continue;
            }

            let annotation_infos = process_annotation_slices(
                query.latest_at,
                num_instances,
                data.class_ids,
                &ent_context.annotations,
            );

            // Has not custom fallback for radius, so we use the default.
            // TODO(andreas): It would be nice to have this handle this fallback as part of the query.
            let radii =
                process_radius_slice(entity_path, num_instances, data.radii, Radius::default());
            let colors = self.process_line_colors(ctx, num_instances, &annotation_infos, &data);
            let world_from_obj = ent_context
                .transform_info
                .single_entity_transform_required(entity_path, "Lines2D");

            let mut line_batch = line_builder
                .batch(entity_path.to_string())
                .depth_offset(ent_context.depth_offset)
                .draw_order(draw_order)
                .always_on_top(is_always_on_top_draw_order(draw_order))
                .world_from_obj(world_from_obj)
                .outline_mask_ids(ent_context.highlight.overall)
                .picking_object_id(re_renderer::PickingLayerObjectId(entity_path.hash64()));

            let mut obj_space_bounding_box = re_math::BoundingBox::NOTHING;

            let mut num_rendered_strips = 0usize;
            for (i, (strip, radius, &color)) in
                itertools::izip!(data.strips.iter(), radii, &colors).enumerate()
            {
                let lines = line_batch
                    .add_strip(strip.iter().copied().map(Into::into))
                    // Looped lines should be connected with rounded corners, so we always add outward extending caps.
                    .flags(LineStripFlags::FLAGS_OUTWARD_EXTENDING_ROUND_CAPS)
                    .color(color)
                    .radius(radius)
                    .picking_instance_id(PickingLayerInstanceId(i as _));

                if let Some(outline_mask_ids) = ent_context
                    .highlight
                    .instances
                    .get(&Instance::from(i as u64))
                {
                    lines.outline_mask_ids(*outline_mask_ids);
                }

                for p in *strip {
                    obj_space_bounding_box.extend((*p).into());
                }

                num_rendered_strips += 1;
            }
            debug_assert_eq!(
                data.strips.len(),
                num_rendered_strips,
                "the number of renderer strips after all post-processing is done should be equal to {} (got {num_rendered_strips} instead)",
                data.strips.len()
            );

            self.data
                .add_bounding_box(entity_path.hash(), obj_space_bounding_box, world_from_obj);

            self.data.ui_labels.extend(process_labels_3d(
                LabeledBatch {
                    entity_path,
                    num_instances,
                    overall_position: obj_space_bounding_box.center(),
                    instance_positions: data.strips.iter().map(|strip| {
                        strip
                            .iter()
                            .copied()
                            .map(glam::Vec3::from)
                            .sum::<glam::Vec3>()
                            / (strip.len() as f32)
                    }),
                    labels: &data.labels,
                    colors: &colors,
                    show_labels: data.show_labels.unwrap_or_else(|| self.fallback_for(ctx)),
                    annotation_infos: &annotation_infos,
                },
                world_from_obj,
            ));
        }
    }

    fn process_line_colors(
        &self,
        ctx: &QueryContext<'_>,
        num_instances: usize,
        annotation_infos: &re_viewer_context::ResolvedAnnotationInfos,
        data: &Lines3DComponentData<'_>,
    ) -> Vec<re_renderer::Color32> {
        if !data.scalar_values.is_empty() {
            let colormap = data.colormap.unwrap_or_else(|| {
                <Self as TypedComponentFallbackProvider<Colormap>>::fallback_for(self, ctx)
            });
            let scalar_range = data
                .scalar_range
                .or_else(|| scalar_range_from_values(data.scalar_values))
                .unwrap_or_else(|| {
                    let range = <Self as TypedComponentFallbackProvider<ValueRange>>::fallback_for(
                        self, ctx,
                    );
                    range.0 .0
                });
            return colormap_scalar_values(
                num_instances,
                data.scalar_values,
                scalar_range,
                colormap,
            );
        }

        process_color_slice(ctx, self, num_instances, annotation_infos, data.colors)
    }
}

#[cfg(test)]
mod always_on_top_tests {
    use super::is_always_on_top_draw_order;

    #[test]
    fn high_draw_order_selects_the_overlay_phase() {
        assert!(is_always_on_top_draw_order(10_000.0));
        assert!(is_always_on_top_draw_order(10_001.0));
        assert!(!is_always_on_top_draw_order(9_999.0));
        assert!(!is_always_on_top_draw_order(f32::NAN));
    }
}

// ---

struct Lines3DComponentData<'a> {
    // Point of views
    strips: Vec<&'a [[f32; 3]]>,

    // Clamped to edge
    colors: &'a [Color],
    radii: &'a [Radius],
    labels: Vec<ArrowString>,
    class_ids: &'a [ClassId],
    scalar_values: &'a [f64],
    scalar_range: Option<[f64; 2]>,
    colormap: Option<Colormap>,

    // Non-repeated
    show_labels: Option<ShowLabels>,
}

impl IdentifiedViewSystem for Lines3DVisualizer {
    fn identifier() -> re_viewer_context::ViewSystemIdentifier {
        "Lines3D".into()
    }
}

impl VisualizerSystem for Lines3DVisualizer {
    fn visualizer_query_info(&self) -> VisualizerQueryInfo {
        VisualizerQueryInfo::from_archetype::<LineStrips3D>()
    }

    fn filter_visualizable_entities(
        &self,
        entities: MaybeVisualizableEntities,
        context: &dyn VisualizableFilterContext,
    ) -> VisualizableEntities {
        re_tracing::profile_function!();
        filter_visualizable_3d_entities(entities, context)
    }

    fn execute(
        &mut self,
        ctx: &ViewContext<'_>,
        view_query: &ViewQuery<'_>,
        context_systems: &ViewContextCollection,
    ) -> Result<Vec<re_renderer::QueueableDrawData>, ViewSystemExecutionError> {
        let mut line_builder = re_renderer::LineDrawableBuilder::new(ctx.viewer_ctx.render_ctx());
        line_builder.radius_boost_in_ui_points_for_outlines(
            re_view::SIZE_BOOST_IN_POINTS_FOR_LINE_OUTLINES,
        );

        use super::entity_iterator::{iter_slices, process_archetype};
        process_archetype::<Self, LineStrips3D, _>(
            ctx,
            view_query,
            context_systems,
            |ctx, spatial_ctx, results| {
                use re_view::RangeResultsExt as _;

                let Some(all_strip_chunks) = results.get_required_chunks(&LineStrip3D::name())
                else {
                    return Ok(());
                };

                let num_strips = all_strip_chunks
                    .iter()
                    .flat_map(|chunk| chunk.iter_slices::<&[[f32; 3]]>(LineStrip3D::name()))
                    .map(|strips| strips.len())
                    .sum();
                if num_strips == 0 {
                    return Ok(());
                }
                line_builder.reserve_strips(num_strips)?;

                let num_vertices = all_strip_chunks
                    .iter()
                    .flat_map(|chunk| chunk.iter_slices::<&[[f32; 3]]>(LineStrip3D::name()))
                    .map(|strips| strips.iter().map(|strip| strip.len()).sum::<usize>())
                    .sum::<usize>();
                line_builder.reserve_vertices(num_vertices)?;

                let timeline = ctx.query.timeline();
                let all_strips_indexed =
                    iter_slices::<&[[f32; 3]]>(&all_strip_chunks, timeline, LineStrip3D::name());
                let all_colors = results.iter_as(timeline, Color::name());
                let all_radii = results.iter_as(timeline, Radius::name());
                let all_labels = results.iter_as(timeline, Text::name());
                let all_class_ids = results.iter_as(timeline, ClassId::name());
                let all_show_labels = results.iter_as(timeline, ShowLabels::name());
                let all_scalar_values = results.iter_as(timeline, Scalar::name());
                let all_scalar_ranges = results.iter_as(timeline, ValueRange::name());
                let all_colormaps = results.iter_as(timeline, Colormap::name());

                let data = re_query::range_zip_1x8(
                    all_strips_indexed,
                    all_colors.slice::<u32>(),
                    all_radii.slice::<f32>(),
                    all_labels.slice::<String>(),
                    all_class_ids.slice::<u16>(),
                    all_show_labels.slice::<bool>(),
                    all_scalar_values.slice::<f64>(),
                    all_scalar_ranges.slice::<[f64; 2]>(),
                    all_colormaps.slice::<u8>(),
                )
                .map(
                    |(
                        _index,
                        strips,
                        colors,
                        radii,
                        labels,
                        class_ids,
                        show_labels,
                        scalar_values,
                        scalar_range,
                        colormap,
                    )| {
                        Lines3DComponentData {
                            strips,
                            colors: colors.map_or(&[], |colors| bytemuck::cast_slice(colors)),
                            radii: radii.map_or(&[], |radii| bytemuck::cast_slice(radii)),
                            labels: labels.unwrap_or_default(),
                            class_ids: class_ids
                                .map_or(&[], |class_ids| bytemuck::cast_slice(class_ids)),
                            scalar_values: scalar_values.unwrap_or_default(),
                            scalar_range: first_copied(scalar_range),
                            colormap: first_copied(colormap).and_then(Colormap::from_u8),
                            show_labels: show_labels
                                .map(|b| !b.is_empty() && b.value(0))
                                .map(Into::into),
                        }
                    },
                );

                self.process_data(ctx, &mut line_builder, view_query, spatial_ctx, data);

                Ok(())
            },
        )?;

        Ok(vec![(line_builder.into_draw_data()?.into())])
    }

    fn data(&self) -> Option<&dyn std::any::Any> {
        Some(self.data.as_any())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn fallback_provider(&self) -> &dyn re_viewer_context::ComponentFallbackProvider {
        self
    }
}

impl TypedComponentFallbackProvider<Color> for Lines3DVisualizer {
    fn fallback_for(&self, ctx: &QueryContext<'_>) -> Color {
        auto_color_for_entity_path(ctx.target_entity_path)
    }
}

impl TypedComponentFallbackProvider<ShowLabels> for Lines3DVisualizer {
    fn fallback_for(&self, ctx: &QueryContext<'_>) -> ShowLabels {
        super::utilities::show_labels_fallback::<LineStrip3D>(ctx)
    }
}

impl TypedComponentFallbackProvider<ValueRange> for Lines3DVisualizer {
    fn fallback_for(&self, _ctx: &QueryContext<'_>) -> ValueRange {
        ValueRange::default()
    }
}

impl TypedComponentFallbackProvider<Colormap> for Lines3DVisualizer {
    fn fallback_for(&self, _ctx: &QueryContext<'_>) -> Colormap {
        Colormap::RedToGreen
    }
}

re_viewer_context::impl_component_fallback_provider!(Lines3DVisualizer => [Color, ShowLabels, ValueRange, Colormap]);

fn colormap_scalar_values(
    num_instances: usize,
    scalar_values: &[f64],
    scalar_range: [f64; 2],
    colormap: Colormap,
) -> Vec<re_renderer::Color32> {
    let [range_min, range_max] = scalar_range;
    let range_width = range_max - range_min;
    let last_value = scalar_values.last().copied().unwrap_or_default();
    let colormap = re_viewer_context::gpu_bridge::colormap_to_re_renderer(colormap);

    (0..num_instances)
        .map(|index| {
            let value = scalar_values.get(index).copied().unwrap_or(last_value);
            let t = if range_width.is_finite() && range_width > 0.0 && value.is_finite() {
                ((value - range_min) / range_width).clamp(0.0, 1.0) as f32
            } else {
                0.0
            };
            let [r, g, b, a] = re_renderer::colormap_srgb(colormap, t);
            re_renderer::Color32::from_rgba_unmultiplied(r, g, b, a)
        })
        .collect()
}

fn scalar_range_from_values(scalar_values: &[f64]) -> Option<[f64; 2]> {
    let mut finite_values = scalar_values
        .iter()
        .copied()
        .filter(|value| value.is_finite());
    let first = finite_values.next()?;
    let (mut min, mut max) = (first, first);
    for value in finite_values {
        min = min.min(value);
        max = max.max(value);
    }
    if min < max {
        Some([min, max])
    } else {
        Some([min, min + 1.0])
    }
}

fn first_copied<T: Copy>(slice: Option<&[T]>) -> Option<T> {
    slice.and_then(|element| element.first()).copied()
}
