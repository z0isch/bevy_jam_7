// CRT Post-Processing Shader
// Creates a retro CRT monitor effect with scanlines, curvature, chromatic aberration, and vignette

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

struct CrtSettings {
    scanline_intensity: f32,
    scanline_count: f32,
    curvature: f32,
    vignette_intensity: f32,
    chromatic_aberration: f32,
    brightness: f32,
    noise_intensity: f32,
    time: f32,
}
@group(0) @binding(2) var<uniform> settings: CrtSettings;

// Simple hash function for noise
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

// Apply CRT barrel distortion / curvature
fn apply_curvature(uv: vec2<f32>, curvature: f32) -> vec2<f32> {
    // Center UV coordinates
    var centered = uv * 2.0 - 1.0;
    
    // Apply barrel distortion
    let barrel = centered * centered;
    centered = centered + centered * barrel * curvature;
    
    // Convert back to 0-1 range
    return centered * 0.5 + 0.5;
}

// Check if UV is outside the screen bounds (for curved edges)
fn is_outside_screen(uv: vec2<f32>) -> bool {
    return uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0;
}

// Scanline effect
fn scanline(uv: vec2<f32>, time: f32, intensity: f32, count: f32) -> f32 {
    // Create scanlines with slight movement for authenticity
    let scanline_pos = uv.y * count + time * 0.5;
    let scanline_wave = sin(scanline_pos * 3.14159265);
    
    // Square the wave for sharper scanlines
    let scanline_strength = scanline_wave * scanline_wave;
    
    // Mix between full brightness and scanline darkness
    return 1.0 - intensity * (1.0 - scanline_strength);
}

// Vignette effect (darkening at screen edges)
fn vignette(uv: vec2<f32>, intensity: f32) -> f32 {
    let centered = uv * 2.0 - 1.0;
    let dist = length(centered);
    
    // Smooth vignette falloff
    return 1.0 - intensity * dist * dist;
}

// Phosphor glow simulation - slightly blur and brighten
fn sample_with_chromatic_aberration(uv: vec2<f32>, aberration: f32) -> vec3<f32> {
    // Sample RGB channels with slight offset for chromatic aberration
    let center = uv - 0.5;
    let dist = length(center);
    let offset = center * dist * aberration;
    
    let r = textureSample(screen_texture, texture_sampler, uv + offset).r;
    let g = textureSample(screen_texture, texture_sampler, uv).g;
    let b = textureSample(screen_texture, texture_sampler, uv - offset).b;
    
    return vec3<f32>(r, g, b);
}

// RGB subpixel pattern (optional, subtle)
fn rgb_subpixel(uv: vec2<f32>, color: vec3<f32>) -> vec3<f32> {
    let pixel_x = fract(uv.x * 1024.0);
    
    // Create subtle RGB subpixel pattern
    var subpixel = vec3<f32>(1.0);
    if (pixel_x < 0.333) {
        subpixel = vec3<f32>(1.0, 0.9, 0.9);
    } else if (pixel_x < 0.666) {
        subpixel = vec3<f32>(0.9, 1.0, 0.9);
    } else {
        subpixel = vec3<f32>(0.9, 0.9, 1.0);
    }
    
    return color * mix(vec3<f32>(1.0), subpixel, 0.15);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Apply CRT curvature
    let curved_uv = apply_curvature(in.uv, settings.curvature);
    
    // Return black if outside screen bounds (creates the curved edge effect)
    if (is_outside_screen(curved_uv)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    
    // Sample with chromatic aberration
    var color = sample_with_chromatic_aberration(curved_uv, settings.chromatic_aberration);
    
    // Apply RGB subpixel pattern
    color = rgb_subpixel(curved_uv, color);
    
    // Apply scanlines
    let scanline_factor = scanline(curved_uv, settings.time, settings.scanline_intensity, settings.scanline_count);
    color = color * scanline_factor;
    
    // Apply vignette
    let vignette_factor = vignette(curved_uv, settings.vignette_intensity);
    color = color * vignette_factor;
    
    // Add subtle noise/static
    let noise = hash(curved_uv * 1000.0 + vec2<f32>(settings.time * 100.0, 0.0));
    color = color + (noise - 0.5) * settings.noise_intensity;
    
    // Apply brightness adjustment
    color = color * settings.brightness;
    
    // Slight phosphor glow (bloom simulation) - brighten bright areas slightly
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    let glow = smoothstep(0.5, 1.0, luminance) * 0.1;
    color = color + glow;
    
    // Clamp to valid range
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    
    return vec4<f32>(color, 1.0);
}

