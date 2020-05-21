pub mod camera;
pub mod figure;
pub mod lod;
pub mod simple;
pub mod terrain;

use self::{
    camera::{Camera, CameraMode},
    figure::FigureMgr,
    lod::{Lod, LodData},
    terrain::Terrain,
};
use crate::{
    anim::character::SkeletonAttr,
    audio::{music::MusicMgr, sfx::SfxMgr, AudioFrontend},
    render::{
        create_pp_mesh, create_skybox_mesh, Consts, Globals, Light, Model, PostProcessLocals,
        PostProcessPipeline, Renderer, Shadow, ShadowLocals, SkyboxLocals, SkyboxPipeline,
    },
    settings::Settings,
    window::{AnalogGameInput, Event},
};
use client::Client;
use common::{
    comp,
    state::State,
    terrain::{BlockKind, TerrainChunk},
    vol::ReadVol,
};
use specs::{Entity as EcsEntity, Join, WorldExt};
use vek::*;

// TODO: Don't hard-code this.
const CURSOR_PAN_SCALE: f32 = 0.005;

const MAX_LIGHT_COUNT: usize = 32;
const MAX_SHADOW_COUNT: usize = 24;
const NUM_DIRECTED_LIGHTS: usize = 2;
const LIGHT_DIST_RADIUS: f32 = 64.0; // The distance beyond which lights may not emit light from their origin
const SHADOW_DIST_RADIUS: f32 = 8.0;
const SHADOW_MAX_DIST: f32 = 96.0; // The distance beyond which shadows may not be visible

// const NEAR_PLANE: f32 = 0.5;
// const FAR_PLANE: f32 = 100000.0;

const SHADOW_NEAR: f32 = 0.25; //1.0; //0.5;//1.0; // Near plane for shadow map rendering.
const SHADOW_FAR: f32 = 128.0; //100000.0;//128.0; //25.0; //100000.0;//25.0; // Far plane for shadow map rendering.

/// Above this speed is considered running
/// Used for first person camera effects
const RUNNING_THRESHOLD: f32 = 0.7;

struct Skybox {
    model: Model<SkyboxPipeline>,
    locals: Consts<SkyboxLocals>,
}

struct PostProcess {
    model: Model<PostProcessPipeline>,
    locals: Consts<PostProcessLocals>,
}

pub struct Scene {
    globals: Consts<Globals>,
    lights: Consts<Light>,
    shadow_mats: Consts<ShadowLocals>,
    shadows: Consts<Shadow>,
    camera: Camera,
    camera_input_state: Vec2<f32>,

    skybox: Skybox,
    postprocess: PostProcess,
    terrain: Terrain<TerrainChunk>,
    pub lod: Lod,
    loaded_distance: f32,
    /// x coordinate is sea level (minimum height for any land chunk), and y
    /// coordinate is the maximum height above the mnimimum for any land
    /// chunk.
    map_bounds: Vec2<f32>,
    select_pos: Option<Vec3<i32>>,

    figure_mgr: FigureMgr,
    sfx_mgr: SfxMgr,
    music_mgr: MusicMgr,
}

pub struct SceneData<'a> {
    pub state: &'a State,
    pub player_entity: specs::Entity,
    pub loaded_distance: f32,
    pub view_distance: u32,
    pub tick: u64,
    pub thread_pool: &'a uvth::ThreadPool,
    pub gamma: f32,
    pub mouse_smoothing: bool,
    pub sprite_render_distance: f32,
    pub figure_lod_render_distance: f32,
    pub is_aiming: bool,
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(renderer: &mut Renderer, client: &Client, settings: &Settings) -> Self {
        let resolution = renderer.get_resolution().map(|e| e as f32);

        Self {
            globals: renderer.create_consts(&[Globals::default()]).unwrap(),
            lights: renderer
                .create_consts(&[Light::default(); MAX_LIGHT_COUNT])
                .unwrap(),
            shadows: renderer
                .create_consts(&[Shadow::default(); MAX_SHADOW_COUNT])
                .unwrap(),
            shadow_mats: renderer
                .create_consts(&[ShadowLocals::default(); MAX_LIGHT_COUNT * 6 + 2])
                .unwrap(),
            camera: Camera::new(resolution.x / resolution.y, CameraMode::ThirdPerson),
            camera_input_state: Vec2::zero(),

            skybox: Skybox {
                model: renderer.create_model(&create_skybox_mesh()).unwrap(),
                locals: renderer.create_consts(&[SkyboxLocals::default()]).unwrap(),
            },
            postprocess: PostProcess {
                model: renderer.create_model(&create_pp_mesh()).unwrap(),
                locals: renderer
                    .create_consts(&[PostProcessLocals::default()])
                    .unwrap(),
            },
            terrain: Terrain::new(renderer),
            lod: Lod::new(renderer, client, settings),
            loaded_distance: 0.0,
            map_bounds: client.world_map.2,
            select_pos: None,

            figure_mgr: FigureMgr::new(),
            sfx_mgr: SfxMgr::new(),
            music_mgr: MusicMgr::new(),
        }
    }

    /// Get a reference to the scene's globals.
    pub fn globals(&self) -> &Consts<Globals> { &self.globals }

    /// Get a reference to the scene's camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Get a reference to the scene's terrain.
    pub fn terrain(&self) -> &Terrain<TerrainChunk> { &self.terrain }

    /// Get a reference to the scene's figure manager.
    pub fn figure_mgr(&self) -> &FigureMgr { &self.figure_mgr }

    /// Get a mutable reference to the scene's camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Set the block position that the player is interacting with
    pub fn set_select_pos(&mut self, pos: Option<Vec3<i32>>) { self.select_pos = pos; }

    pub fn select_pos(&self) -> Option<Vec3<i32>> { self.select_pos }

    /// Handle an incoming user input event (e.g.: cursor moved, key pressed,
    /// window closed).
    ///
    /// If the event is handled, return true.
    pub fn handle_input_event(&mut self, event: Event) -> bool {
        match event {
            // When the window is resized, change the camera's aspect ratio
            Event::Resize(dims) => {
                self.camera.set_aspect_ratio(dims.x as f32 / dims.y as f32);
                true
            },
            // Panning the cursor makes the camera rotate
            Event::CursorPan(delta) => {
                self.camera.rotate_by(Vec3::from(delta) * CURSOR_PAN_SCALE);
                true
            },
            // Zoom the camera when a zoom event occurs
            Event::Zoom(delta) => {
                self.camera
                    .zoom_switch(delta * (0.05 + self.camera.get_distance() * 0.01));
                true
            },
            Event::AnalogGameInput(input) => match input {
                AnalogGameInput::CameraX(d) => {
                    self.camera_input_state.x = d;
                    true
                },
                AnalogGameInput::CameraY(d) => {
                    self.camera_input_state.y = d;
                    true
                },
                _ => false,
            },
            // All other events are unhandled
            _ => false,
        }
    }

    /// Maintain data such as GPU constant buffers, models, etc. To be called
    /// once per tick.
    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        audio: &mut AudioFrontend,
        scene_data: &SceneData,
    ) {
        // Get player position.
        let ecs = scene_data.state.ecs();

        let player_pos = ecs
            .read_storage::<comp::Pos>()
            .get(scene_data.player_entity)
            .map_or(Vec3::zero(), |pos| pos.0);

        let player_rolling = ecs
            .read_storage::<comp::CharacterState>()
            .get(scene_data.player_entity)
            .map_or(false, |cs| cs.is_dodge());

        let is_running = ecs
            .read_storage::<comp::Vel>()
            .get(scene_data.player_entity)
            .map(|v| v.0.magnitude_squared() > RUNNING_THRESHOLD.powi(2))
            .unwrap_or(false);

        let on_ground = ecs
            .read_storage::<comp::PhysicsState>()
            .get(scene_data.player_entity)
            .map(|p| p.on_ground);

        let player_scale = match scene_data
            .state
            .ecs()
            .read_storage::<comp::Body>()
            .get(scene_data.player_entity)
        {
            Some(comp::Body::Humanoid(body)) => SkeletonAttr::calculate_scale(body),
            _ => 1_f32,
        };

        // Add the analog input to camera
        self.camera
            .rotate_by(Vec3::from([self.camera_input_state.x, 0.0, 0.0]));
        self.camera
            .rotate_by(Vec3::from([0.0, self.camera_input_state.y, 0.0]));

        // Alter camera position to match player.
        let tilt = self.camera.get_orientation().y;
        let dist = self.camera.get_distance();

        let up = match self.camera.get_mode() {
            CameraMode::FirstPerson => {
                if player_rolling {
                    player_scale * 0.8
                } else if is_running && on_ground.unwrap_or(false) {
                    player_scale * 1.65 + (scene_data.state.get_time() as f32 * 17.0).sin() * 0.05
                } else {
                    player_scale * 1.65
                }
            },
            CameraMode::ThirdPerson if scene_data.is_aiming => player_scale * 2.1,
            CameraMode::ThirdPerson => player_scale * 1.65,
        };

        self.camera
            .set_focus_pos(player_pos + Vec3::unit_z() * (up - tilt.min(0.0).sin() * dist * 0.6));

        // Tick camera for interpolation.
        self.camera.update(
            scene_data.state.get_time(),
            scene_data.state.get_delta_time(),
            scene_data.mouse_smoothing,
        );

        // Compute camera matrices.
        self.camera.compute_dependents(&*scene_data.state.terrain());
        let camera::Dependents {
            view_mat,
            proj_mat,
            cam_pos,
        } = self.camera.dependents();

        // Update chunk loaded distance smoothly for nice shader fog
        self.loaded_distance =
            (0.98 * self.loaded_distance + 0.02 * scene_data.loaded_distance).max(0.01);

        // Update light constants
        let mut lights = (
            &scene_data.state.ecs().read_storage::<comp::Pos>(),
            scene_data.state.ecs().read_storage::<comp::Ori>().maybe(),
            scene_data
                .state
                .ecs()
                .read_storage::<crate::ecs::comp::Interpolated>()
                .maybe(),
            &scene_data
                .state
                .ecs()
                .read_storage::<comp::LightAnimation>(),
        )
            .join()
            .filter(|(pos, _, _, _)| {
                (pos.0.distance_squared(player_pos) as f32)
                    < self.loaded_distance.powf(2.0) + LIGHT_DIST_RADIUS
            })
            .map(|(pos, ori, interpolated, light_anim)| {
                // Use interpolated values if they are available
                let (pos, ori) =
                    interpolated.map_or((pos.0, ori.map(|o| o.0)), |i| (i.pos, Some(i.ori)));
                let rot = {
                    if let Some(o) = ori {
                        Mat3::rotation_z(-o.x.atan2(o.y))
                    } else {
                        Mat3::identity()
                    }
                };
                Light::new(
                    pos + (rot * light_anim.offset),
                    light_anim.col,
                    light_anim.strength,
                )
            })
            .collect::<Vec<_>>();
        lights.sort_by_key(|light| light.get_pos().distance_squared(player_pos) as i32);
        lights.truncate(MAX_LIGHT_COUNT);
        renderer
            .update_consts(&mut self.lights, &lights)
            .expect("Failed to update light constants");

        // Update shadow constants
        let mut shadows = (
            &scene_data.state.ecs().read_storage::<comp::Pos>(),
            scene_data
                .state
                .ecs()
                .read_storage::<crate::ecs::comp::Interpolated>()
                .maybe(),
            scene_data.state.ecs().read_storage::<comp::Scale>().maybe(),
            &scene_data.state.ecs().read_storage::<comp::Body>(),
            &scene_data.state.ecs().read_storage::<comp::Stats>(),
        )
            .join()
            .filter(|(_, _, _, _, stats)| !stats.is_dead)
            .filter(|(pos, _, _, _, _)| {
                (pos.0.distance_squared(player_pos) as f32)
                    < (self.loaded_distance.min(SHADOW_MAX_DIST) + SHADOW_DIST_RADIUS).powf(2.0)
            })
            .map(|(pos, interpolated, scale, _, _)| {
                Shadow::new(
                    // Use interpolated values pos if it is available
                    interpolated.map_or(pos.0, |i| i.pos),
                    scale.map_or(1.0, |s| s.0),
                )
            })
            .collect::<Vec<_>>();
        shadows.sort_by_key(|shadow| shadow.get_pos().distance_squared(player_pos) as i32);
        shadows.truncate(MAX_SHADOW_COUNT);
        renderer
            .update_consts(&mut self.shadows, &shadows)
            .expect("Failed to update light constants");

        // Update light projection matrices for the shadow map.
        let time_of_day = scene_data.state.get_time_of_day();
        let shadow_res = renderer.get_shadow_resolution();
        // NOTE: The aspect ratio is currently always 1 for our cube maps, since they
        // are equal on all sides.
        let shadow_aspect = shadow_res.x as f32 / shadow_res.y as f32;
        // First, add a projected matrix for our directed hard lights.
        let directed_view_proj = Mat4::orthographic_rh_no(FrustumPlanes {
            left: -(shadow_res.x as f32) / 2.0,
            right: shadow_res.x as f32 / 2.0,
            bottom: -(shadow_res.y as f32) / 2.0,
            top: shadow_res.y as f32 / 2.0,
            near: SHADOW_NEAR,
            far: SHADOW_FAR,
        });
        // Construct matrices to transform from world space to light space for the sun
        // and moon.
        const TIME_FACTOR: f32 = (std::f32::consts::PI * 2.0) / (3600.0 * 24.0);
        let angle_rad = time_of_day as f32 * TIME_FACTOR;
        let sun_dir = Vec3::new(angle_rad.sin(), 0.0, angle_rad.cos());
        let moon_dir = Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos() - 0.5);
        let sun_view_mat = Mat4::model_look_at_rh(-sun_dir, Vec3::zero(), Vec3::up());
        let moon_view_mat = Mat4::model_look_at_rh(-moon_dir, Vec3::zero(), Vec3::up());
        // Now, construct the full projection matrices in the first two directed light
        // slots.
        let mut shadow_mats = Vec::with_capacity(6 * (lights.len() + 1));
        shadow_mats.push(ShadowLocals::new(directed_view_proj * sun_view_mat));
        shadow_mats.push(ShadowLocals::new(directed_view_proj * moon_view_mat));
        // This leaves us with four dummy slots, which we push as defaults.
        shadow_mats.extend_from_slice(&[ShadowLocals::default(); 6 - NUM_DIRECTED_LIGHTS] as _);
        // Now, we tackle point lights.
        // First, create a perspective projection matrix at 90 degrees (to cover a whole
        // face of the cube map we're using).
        let shadow_proj =
            Mat4::perspective_rh_no(90.0f32.to_radians(), shadow_aspect, SHADOW_NEAR, SHADOW_FAR);
        // Next, construct the 6 orientations we'll use for the six faces, in terms of
        // their (forward, up) vectors.
        let orientations = [
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),
            (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),
            (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
            (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)),
            (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0)),
            (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)),
        ];
        // NOTE: We could create the shadow map collection at the same time as the
        // lights, but then we'd have to sort them both, which wastes time.  Plus, we
        // want to prepend our directed lights.
        shadow_mats.extend(lights.iter().flat_map(|light| {
            // Now, construct the full projection matrix by making the light look at each
            // cube face.
            let eye = Vec3::new(light.pos[0], light.pos[1], light.pos[2]);
            orientations.iter().map(move |&(forward, up)| {
                ShadowLocals::new(shadow_proj * Mat4::look_at_rh(eye, eye + forward, up))
            })
        }));
        /* shadow_mats.push(
                    Mat4::orthographic_rh_no
        float near_plane = 1.0f, far_plane = 7.5f;
        glm::mat4 lightProjection = glm::ortho(-10.0f, 10.0f, -10.0f, 10.0f, near_plane, far_plane);

                ); */
        renderer
            .update_consts(&mut self.shadow_mats, &shadow_mats)
            .expect("Failed to update light constants");

        // Update global constants.
        renderer
            .update_consts(&mut self.globals, &[Globals::new(
                view_mat,
                proj_mat,
                cam_pos,
                self.camera.get_focus_pos(),
                self.loaded_distance,
                self.lod.get_data().tgt_detail as f32,
                self.map_bounds,
                time_of_day,
                scene_data.state.get_time(),
                renderer.get_resolution(),
                Vec2::new(SHADOW_NEAR, SHADOW_FAR),
                lights.len(),
                shadows.len(),
                NUM_DIRECTED_LIGHTS,
                scene_data
                    .state
                    .terrain()
                    .get(cam_pos.map(|e| e.floor() as i32))
                    .map(|b| b.kind())
                    .unwrap_or(BlockKind::Air),
                self.select_pos,
                scene_data.gamma,
                self.camera.get_mode(),
                scene_data.sprite_render_distance as f32 - 20.0,
            )])
            .expect("Failed to update global constants");

        // Maintain LoD.
        self.lod.maintain(renderer, time_of_day);

        // Maintain the terrain.
        self.terrain.maintain(
            renderer,
            &scene_data,
            self.camera.get_focus_pos(),
            self.loaded_distance,
            view_mat,
            proj_mat,
        );

        // Maintain the figures.
        self.figure_mgr.maintain(renderer, scene_data, &self.camera);

        // Remove unused figures.
        self.figure_mgr.clean(scene_data.tick);

        // Maintain audio
        self.sfx_mgr
            .maintain(audio, scene_data.state, scene_data.player_entity);
        self.music_mgr.maintain(audio, scene_data.state);
    }

    /// Render the scene using the provided `Renderer`.
    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        state: &State,
        player_entity: EcsEntity,
        tick: u64,
        scene_data: &SceneData,
    ) {
        // Render terrain and figures.
        self.terrain.render(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            &self.shadow_mats,
            self.lod.get_data(),
            self.camera.get_focus_pos(),
        );
        self.figure_mgr.render(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            self.lod.get_data(),
            &self.camera,
            scene_data.figure_lod_render_distance,
        );
        self.lod.render(renderer, &self.globals);

        // Render the skybox.
        let lod = self.lod.get_data();
        renderer.render_skybox(
            &self.skybox.model,
            &self.globals,
            &self.skybox.locals,
            &lod.map,
            &lod.horizon,
        );

        self.figure_mgr.render_player(
            renderer,
            state,
            player_entity,
            tick,
            &self.globals,
            &self.lights,
            &self.shadows,
            lod,
            &self.camera,
            scene_data.figure_lod_render_distance,
        );

        self.terrain.render_translucent(
            renderer,
            &self.globals,
            &self.lights,
            &self.shadows,
            lod,
            self.camera.get_focus_pos(),
            scene_data.sprite_render_distance,
        );

        renderer.render_post_process(
            &self.postprocess.model,
            &self.globals,
            &self.postprocess.locals,
        );
    }
}
