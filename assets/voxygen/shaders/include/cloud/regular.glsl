#include <random.glsl>
#include <lod.glsl>

const float CLOUD_THRESHOLD = 0.27;
const float CLOUD_SCALE = 5.0;
const float CLOUD_DENSITY = 150.0;

vec2 get_cloud_heights(vec2 pos) {
    const float CLOUD_HALF_WIDTH = 300;
    const float CLOUD_HEIGHT_VARIATION = 1500.0;
    float cloud_alt = CLOUD_AVG_ALT + (texture(t_noise, pos.xy * 0.00005).x - 0.5) * CLOUD_HEIGHT_VARIATION;
    #if (CLOUD_MODE > CLOUD_MODE_MINIMAL)
        cloud_alt += (texture(t_noise, pos.xy * 0.001).x - 0.5) * 0.1 * CLOUD_HEIGHT_VARIATION;
    #endif
    return vec2(cloud_alt, CLOUD_HALF_WIDTH);
}

float emission_strength = clamp((sin(time_of_day.x / (3600 * 24)) - 0.8) / 0.1, 0, 1);

// Returns vec4(r, g, b, density)
vec4 cloud_at(vec3 pos, float dist, out vec3 emission) {
    // Natural attenuation of air (air naturally attenuates light that passes through it)
    // Simulate the atmosphere thinning as you get higher. Not physically accurate, but then
    // it can't be since Veloren's world is flat, not spherical.
    float air = 0.00035 * clamp((10000.0 - pos.z) / 7000, 0, 1);

    // Mist sits close to the ground in valleys (TODO: use base_alt to put it closer to water)
    float mist_min_alt = 0.5;
    #if (CLOUD_MODE > CLOUD_MODE_LOW)
        mist_min_alt = (texture(t_noise, pos.xy * 0.00015).x - 0.5) * 1.25 + 0.5;
    #endif
    mist_min_alt *= 250;
    const float MIST_FADE_HEIGHT = 500;
    float mist = 0.00125 * pow(clamp(1.0 - (pos.z - mist_min_alt) / MIST_FADE_HEIGHT, 0.0, 1), 4.0) / (1.0 + pow(1.0 + dist / 20000.0, 2.0));

    vec3 wind_pos = vec3(pos.xy + wind_offset, pos.z);

    // Clouds
    float cloud_tendency = cloud_tendency_at(pos.xy);
    float cloud = 0;

    vec2 cloud_attr = get_cloud_heights(wind_pos.xy);
    float cloud_factor = 0.0;
    float turb_noise = 0.0;
    float sun_access = 0.0;
    float moon_access = 0.0;
    float cloud_sun_access = 0.0;
    float cloud_moon_access = 0.0;
    // This is a silly optimisation but it actually nets us a fair few fps by skipping quite a few expensive calcs
    if (cloud_tendency > 0 || mist > 0.0) {
        // Turbulence (small variations in clouds/mist)
        const float turb_speed = -1.0; // Turbulence goes the opposite way
        vec3 turb_offset = vec3(1, 1, 0) * time_of_day.x * turb_speed;
        #if (CLOUD_MODE >= CLOUD_MODE_MINIMAL)
            turb_noise = noise_3d((wind_pos + turb_offset) * 0.001) - 0.5;
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
            turb_noise += (noise_3d((wind_pos + turb_offset * 0.3) * 0.004) - 0.5) * 0.35;
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
            turb_noise += (noise_3d((wind_pos + turb_offset * 0.3) * 0.01) - 0.5) * 0.125;
        #endif
        mist *= 1.0 + turb_noise;

        cloud_factor = 0.25 * (1.0 - pow(min(abs(pos.z - cloud_attr.x) / (cloud_attr.y * pow(max(cloud_tendency * 20.0, 0), 0.5)), 1.0), 1.0));
        float cloud_flat = min(cloud_tendency, 0.07) * 0.05;
        cloud_flat *= (1.0 + turb_noise * 7.0 * max(0, 1.0 - cloud_factor * 5));
        cloud = cloud_flat * pow(cloud_factor, 2) * 20;

        // What proportion of sunlight is *not* being blocked by nearby cloud? (approximation)
        cloud_sun_access = clamp((pos.z - cloud_attr.x + turb_noise * 250.0) * 0.002 + 0.35, 0, 1);
        // Since we're assuming the sun/moon is always above (not always correct) it's the same for the moon
        cloud_moon_access = sun_access;

        #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
            // Try to calculate a reasonable approximation of the cloud normal
            float cloud_tendency_x = cloud_tendency_at(pos.xy + vec2(100, 0));
            float cloud_tendency_y = cloud_tendency_at(pos.xy + vec2(0, 100));
            vec3 cloud_norm = vec3(
                (cloud_tendency - cloud_tendency_x) * 4,
                (cloud_tendency - cloud_tendency_y) * 4,
                (pos.z - cloud_attr.x) / 250 + turb_noise + 0.5
            );
            cloud_sun_access = mix(max(dot(-sun_dir.xyz, cloud_norm) + 0.0, 0.025), cloud_sun_access, 0.25);
            cloud_moon_access = mix(max(dot(-moon_dir.xyz, cloud_norm) + 0.35, 0.025), cloud_moon_access, 0.25);
        #endif
    }

    float mist_sun_access = 0.5 + turb_noise * 0.5;
    float mist_moon_access = mist_sun_access;
    sun_access = mix(cloud_sun_access, mist_sun_access, clamp(mist * 20000, 0, 1));
    moon_access = mix(cloud_moon_access, mist_moon_access, clamp(mist * 20000, 0, 1));

    // Prevent mist (i.e: vapour beneath clouds) being accessible to the sun to avoid visual problems
    //float suppress_mist = clamp((pos.z - cloud_attr.x + cloud_attr.y) / 300, 0, 1);
    //sun_access *= suppress_mist;
    //moon_access *= suppress_mist;

    // Prevent clouds and mist appearing underground (but fade them out gently)
    float not_underground = clamp(1.0 - (alt_at(pos.xy - focus_off.xy) - (pos.z - focus_off.z)) / 80.0, 0, 1);
    air *= not_underground;
    float vapor_density = (mist + cloud) * not_underground;

    if (emission_strength <= 0.0) {
        emission = vec3(0);
    } else {
        float z = clamp(pos.z, 0, 10000);
        float emission_alt = 4000.0;
        #if (CLOUD_MODE >= CLOUD_MODE_LOW)
            emission_alt += (noise_3d(vec3(wind_pos.xy * 0.00003 + cloud_tendency * 0.2, time_of_day.x * 0.0001)) - 0.5) * 6000;
        #endif
        #if (CLOUD_MODE >= CLOUD_MODE_HIGH)
            emission_alt += (noise_3d(vec3(wind_pos.xy * 0.0005 + cloud_tendency * 0.2, emission_alt * 0.0001 + time_of_day.x * 0.0005)) - 0.5) * 1000;
        #endif
        float tail = (texture(t_noise, wind_pos.xy * 0.00005).x - 0.5) * 10 + (z - emission_alt) * 0.001;
        vec3 emission_col = vec3(0.6 + tail * 0.6, 1.0, 0.3 + tail * 0.2);
        float emission_nz = max(texture(t_noise, wind_pos.xy * 0.00003).x - 0.6, 0) / (10.0 + abs(z - emission_alt) / 40);
        #if (CLOUD_MODE >= CLOUD_MODE_MEDIUM)
            emission_nz *= (1.0 + (noise_3d(vec3(wind_pos.xy * 0.05, time_of_day.x * 0.15) * 0.004) - 0.5) * 4.0);
        #endif
        emission = emission_col * emission_nz * emission_strength * max(sun_dir.z, 0) * 20;
    }

    // We track vapor density and air density separately. Why? Because photons will ionize particles in air
    // leading to rayleigh scattering, but water vapor will not. Tracking these indepedently allows us to
    // get more correct colours.
    return vec4(sun_access, moon_access, vapor_density, air);
}

float atan2(in float y, in float x) {
    bool s = (abs(x) > abs(y));
    return mix(PI/2.0 - atan(x,y), atan(y,x), s);
}

const float DIST_CAP = 50000;
#if (CLOUD_MODE == CLOUD_MODE_ULTRA)
    const uint QUALITY = 200u;
#elif (CLOUD_MODE == CLOUD_MODE_HIGH)
    const uint QUALITY = 50u;
#elif (CLOUD_MODE == CLOUD_MODE_MEDIUM)
    const uint QUALITY = 30u;
#elif (CLOUD_MODE == CLOUD_MODE_LOW)
    const uint QUALITY = 16u;
#elif (CLOUD_MODE == CLOUD_MODE_MINIMAL)
    const uint QUALITY = 5u;
#endif

const float STEP_SCALE = DIST_CAP / (10.0 * float(QUALITY));

float step_to_dist(float step, float quality) {
    return pow(step, 2) * STEP_SCALE / quality;
}

float dist_to_step(float dist, float quality) {
    return pow(dist / STEP_SCALE * quality, 0.5);
}

vec3 get_cloud_color(vec3 surf_color, vec3 dir, vec3 origin, const float time_of_day, float max_dist, const float quality) {
    // Limit the marching distance to reduce maximum jumps
    max_dist = min(max_dist, DIST_CAP);

    origin.xyz += focus_off.xyz;

    // This hack adds a little direction-dependent noise to clouds. It's not correct, but it very cheaply
    // improves visual quality for low cloud settings
    float splay = 1.0;
    vec3 dir_diff = vec3(0);
    #if (CLOUD_MODE == CLOUD_MODE_MINIMAL)
        /* splay += (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.5 - time_of_day * 0.000025).x - 0.5) * 0.4 / (1.0 + pow(dir.z, 2) * 10); */
        /* dir_diff = vec3( */
        /*     (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10), */
        /*     (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10), */
        /*     (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 1.0 - time_of_day * 0.00005).x - 0.5) * 0.2 / (1.0 + pow(dir.z, 2) * 10) */
        /* ) * 1500; */
        splay += (texture(t_noise, vec2(atan2(dir.x, dir.y) * 2 / PI, dir.z) * 5.0 - time_of_day * 0.00005).x - 0.5) * 0.075 / (1.0 + pow(dir.z, 2) * 10);
    #endif

    // Proportion of sunlight that get scattered back into the camera by clouds
    float sun_scatter = max(dot(-dir, sun_dir.xyz), 0.5);
    float moon_scatter = max(dot(-dir, moon_dir.xyz), 0.5);
    vec3 sky_color = get_sky_color();
    float net_light = get_sun_brightness() + get_moon_brightness();

    float cdist = max_dist;
    float ldist = cdist;
    // i is an emergency brake
    float min_dist = clamp(max_dist / 4, 0.25, 24);
    for (int i = 0; cdist > min_dist && i < 250; i ++) {
        ldist = cdist;
        cdist = step_to_dist(trunc(dist_to_step(cdist - 0.25, quality)), quality);

        vec3 emission;
        vec4 sample = cloud_at(origin + (dir + dir_diff / ldist) * ldist * splay, cdist, emission);

        vec2 density_integrals = max(sample.zw, vec2(0)) * (ldist - cdist);

        float sun_access = sample.x;
        float moon_access = sample.y;
        float scatter_factor = 1.0 - 1.0 / (1.0 + density_integrals.x);

        const float RAYLEIGH = 0.25;

        surf_color =
            // Attenuate light passing through the clouds
            surf_color * (1.0 - scatter_factor) +
            // This is not rayleigh scattering, but it's good enough for our purposes (only considers sun)
            (1.0 - surf_color) * net_light * sky_color * density_integrals.y * RAYLEIGH +
            // Add the directed light light scattered into the camera by the clouds
            get_sun_color() * sun_scatter * sun_access * scatter_factor * get_sun_brightness() +
            get_moon_color() * moon_scatter * moon_access * scatter_factor * get_moon_brightness() +
            emission * density_integrals.y +
            // Global illumination (uniform scatter from the sky)
            sky_color * sun_access * scatter_factor * get_sun_brightness() +
            sky_color * moon_access * scatter_factor * get_moon_brightness();
    }

    return surf_color;
}
