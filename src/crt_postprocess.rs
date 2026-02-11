//! CRT Post-Processing Effect
//!
//! Adds a retro CRT monitor effect with scanlines, curvature, chromatic aberration, and vignette.

use bevy::{
    core_pipeline::{
        FullscreenShader,
        core_2d::graph::{Core2d, Node2d},
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::ViewTarget,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/crt.wgsl";

/// Plugin that adds CRT post-processing effect to 2D cameras
pub struct CrtPostProcessPlugin;

impl Plugin for CrtPostProcessPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<CrtSettings>::default(),
            UniformComponentPlugin::<CrtSettings>::default(),
        ));
        app.add_systems(Update, update_crt_time);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_crt_pipeline);

        render_app
            .add_render_graph_node::<ViewNodeRunner<CrtPostProcessNode>>(
                Core2d,
                CrtPostProcessLabel,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    CrtPostProcessLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CrtPostProcessLabel;

#[derive(Default)]
struct CrtPostProcessNode;

impl ViewNode for CrtPostProcessNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CrtSettings,
        &'static DynamicUniformIndex<CrtSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, _crt_settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let crt_pipeline = world.resource::<CrtPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(crt_pipeline.pipeline_id) else {
            return Ok(());
        };

        let settings_uniforms = world.resource::<ComponentUniforms<CrtSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "crt_bind_group",
            &pipeline_cache.get_bind_group_layout(&crt_pipeline.layout),
            &BindGroupEntries::sequential((
                post_process.source,
                &crt_pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("crt_post_process_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct CrtPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_crt_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "crt_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<CrtSettings>(true),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let shader = asset_server.load(SHADER_ASSET_PATH);
    let vertex_state = fullscreen_shader.to_vertex_state();

    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("crt_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: vertex_state,
        fragment: Some(FragmentState {
            shader,
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });

    commands.insert_resource(CrtPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

/// Settings for the CRT post-processing effect.
/// Add this component to a Camera2d to enable the effect.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct CrtSettings {
    /// Scanline intensity (0.0 = no scanlines, 1.0 = full intensity)
    pub scanline_intensity: f32,
    /// Scanline count (higher = more scanlines)
    pub scanline_count: f32,
    /// CRT screen curvature (0.0 = flat, higher = more curved)
    pub curvature: f32,
    /// Vignette intensity (0.0 = no vignette, 1.0 = strong vignette)
    pub vignette_intensity: f32,
    /// Chromatic aberration strength
    pub chromatic_aberration: f32,
    /// Screen brightness
    pub brightness: f32,
    /// Noise/static intensity
    pub noise_intensity: f32,
    /// Time for animated effects (scanline flicker, noise)
    pub time: f32,
}

impl Default for CrtSettings {
    fn default() -> Self {
        Self {
            scanline_intensity: 0.3,
            scanline_count: 300.0,
            curvature: 0.05,
            vignette_intensity: 0.5,
            chromatic_aberration: 0.02,
            brightness: 1.,
            noise_intensity: 0.003,
            time: 0.0,
        }
    }
}

/// System to update the time uniform for animated CRT effects
pub fn update_crt_time(time: Res<Time>, mut settings: Query<&mut CrtSettings>) {
    for mut setting in &mut settings {
        setting.time = time.elapsed_secs();
    }
}
