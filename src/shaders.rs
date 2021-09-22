
pub const GLOBAL_COMMON: &'static str = include_str!("shaders/global.common.glsl");

pub const COLOR_2D_VERT: &'static str = include_str!("shaders/color_2d.vert.glsl");
pub const COLOR_3D_VERT: &'static str = include_str!("shaders/color_3d.vert.glsl");
pub const COLOR_3D_INSTANCED_TRANFORM_VERT: &'static str = include_str!("shaders/color_3d_instanced_transform.vert.glsl");
pub const FULLSCREEN_QUAD_VERT: &'static str = include_str!("shaders/fullscreen_quad.vert.glsl");

pub const FLAT_COLOR_FRAG: &'static str = include_str!("shaders/flat_color.frag.glsl");
pub const BASIC_TEXTURED_FRAG: &'static str = include_str!("shaders/basic_textured.frag.glsl");


pub const TEST_POST_EFFECT_COMPUTE: &'static str = include_str!("shaders/test_post_effect.compute.glsl");
