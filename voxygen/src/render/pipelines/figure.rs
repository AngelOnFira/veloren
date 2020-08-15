use super::{
    super::{Mesh, Model, Pipeline, TerrainPipeline, TgtColorFmt, TgtDepthStencilFmt},
    shadow, Globals, Light, Shadow,
};
use crate::mesh::greedy::GreedyMesh;
use core::ops::Range;
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta, state::ColorMask,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos_norm: u32 = "v_pos_norm",
        col: u32 = "v_col",
        // BBBBBBAA
        // B = Bone
        // A = AO
        ao_bone: u8 = "v_ao_bone",
    }

    constant Locals {
        model_mat: [[f32; 4]; 4] = "model_mat",
        model_col: [f32; 4] = "model_col",
        atlas_offs: [i32; 4] = "atlas_offs",
        model_pos: [f32; 3] = "model_pos",
        flags: u32 = "flags",
    }

    constant BoneData {
        bone_mat: [[f32; 4]; 4] = "bone_mat",
        normals_mat: [[f32; 4]; 4] = "normals_mat",
    }

    pipeline pipe {
        // vbuf: gfx::VertexBuffer<Vertex> = (),
        vbuf: gfx::VertexBuffer<<TerrainPipeline as Pipeline>::Vertex> = (),
        // abuf: gfx::VertexBuffer<<TerrainPipeline as Pipeline>::Vertex> = (),
        col_lights: gfx::TextureSampler<[f32; 4]> = "t_col_light",

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        bones: gfx::ConstantBuffer<BoneData> = "u_bones",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        point_shadow_maps: gfx::TextureSampler<f32> = "t_point_shadow_maps",
        directed_shadow_maps: gfx::TextureSampler<f32> = "t_directed_shadow_maps",

        alt: gfx::TextureSampler<[f32; 2]> = "t_alt",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<shadow::Locals> = "u_light_shadows",

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth_stencil: gfx::DepthTarget<TgtDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
        // tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Replace))),
    }
}

impl Locals {
    pub fn new(
        model_mat: anim::vek::Mat4<f32>,
        col: Rgba<f32>,
        pos: anim::vek::Vec3<f32>,
        atlas_offs: Vec2<i32>,
        is_player: bool,
    ) -> Self {
        let mut flags = 0;
        flags |= is_player as u32;

        Self {
            model_mat: model_mat.into_col_arrays(),
            model_col: col.into_array(),
            model_pos: pos.into_array(),
            atlas_offs: Vec4::from(atlas_offs).into_array(),
            flags,
        }
    }
}

impl Default for Locals {
    fn default() -> Self {
        Self::new(
            anim::vek::Mat4::identity(),
            Rgba::broadcast(1.0),
            anim::vek::Vec3::default(),
            Vec2::default(),
            false,
        )
    }
}

impl BoneData {
    pub fn new(bone_mat: anim::vek::Mat4<f32>, normals_mat: anim::vek::Mat4<f32>) -> Self {
        Self {
            bone_mat: bone_mat.into_col_arrays(),
            normals_mat: normals_mat.into_col_arrays(),
        }
    }
}

impl Default for BoneData {
    fn default() -> Self { Self::new(anim::vek::Mat4::identity(), anim::vek::Mat4::identity()) }
}

pub struct FigurePipeline;

impl Pipeline for FigurePipeline {
    type Vertex = Vertex;
}

pub struct FigureModel {
    pub opaque: Model<TerrainPipeline>,
    /* TODO: Consider using mipmaps instead of storing multiple texture atlases for different
     * LOD levels. */
}

impl FigureModel {
    /// Start a greedy mesh designed for figure bones.
    pub fn make_greedy<'a>() -> GreedyMesh<'a> {
        // NOTE: Required because we steal two bits from the normal in the shadow uint
        // in order to store the bone index.  The two bits are instead taken out
        // of the atlas coordinates, which is why we "only" allow 1 << 15 per
        // coordinate instead of 1 << 16.
        let max_size = guillotiere::Size::new((1 << 15) - 1, (1 << 15) - 1);
        GreedyMesh::new(max_size)
    }
}

pub type BoneMeshes = (Mesh<TerrainPipeline>, (anim::vek::Aabb<f32>, Range<usize>));
