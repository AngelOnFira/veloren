#include <random.glsl>
#include <sky.glsl>
#include <srgb.glsl>

uniform sampler2D t_map;
uniform sampler2D t_horizon;

vec2 pos_to_uv(vec2 pos) {
	vec2 uv_pos = (pos + 16) / 32768.0;
	return vec2(uv_pos.x, 1.0 - uv_pos.y);
}

// textureBicubic from https://stackoverflow.com/a/42179924
vec4 cubic(float v) {
    vec4 n = vec4(1.0, 2.0, 3.0, 4.0) - v;
    vec4 s = n * n * n;
    float x = s.x;
    float y = s.y - 4.0 * s.x;
    float z = s.z - 4.0 * s.y + 6.0 * s.x;
    float w = 6.0 - x - y - z;
    return vec4(x, y, z, w) * (1.0/6.0);
}

vec4 textureBicubic(sampler2D sampler, vec2 texCoords) {
   vec2 texSize = textureSize(sampler, 0);
   vec2 invTexSize = 1.0 / texSize;

   texCoords = texCoords * texSize - 0.5;


    vec2 fxy = fract(texCoords);
    texCoords -= fxy;

    vec4 xcubic = cubic(fxy.x);
    vec4 ycubic = cubic(fxy.y);

    vec4 c = texCoords.xxyy + vec2 (-0.5, +1.5).xyxy;

    vec4 s = vec4(xcubic.xz + xcubic.yw, ycubic.xz + ycubic.yw);
    vec4 offset = c + vec4 (xcubic.yw, ycubic.yw) / s;

    offset *= invTexSize.xxyy;

    vec4 sample0 = texture(sampler, offset.xz);
    vec4 sample1 = texture(sampler, offset.yz);
    vec4 sample2 = texture(sampler, offset.xw);
    vec4 sample3 = texture(sampler, offset.yw);

    float sx = s.x / (s.x + s.y);
    float sy = s.z / (s.z + s.w);

    return mix(
       mix(sample3, sample2, sx), mix(sample1, sample0, sx)
    , sy);
}

float alt_at(vec2 pos) {
	return texture/*textureBicubic*/(t_map, pos_to_uv(pos)).a * (/*1300.0*//*1278.7266845703125*/view_distance.w) + /*140.0*/view_distance.z;
		//+ (texture(t_noise, pos * 0.002).x - 0.5) * 64.0;

	return 0.0
		+ pow(texture(t_noise, pos * 0.00005).x * 1.4, 3.0) * 1000.0
		+ texture(t_noise, pos * 0.001).x * 100.0
		+ texture(t_noise, pos * 0.003).x * 30.0;
}

float horizon_at2(vec4 f_horizons, float alt, vec3 pos, /*float time_of_day*/vec3 light_dir) {
    // vec3 sun_dir = get_sun_dir(time_of_day);
    const float PI_2 = 3.1415926535897932384626433832795 / 2.0;
    const float MIN_LIGHT = 0.0;//0.115/*0.0*/;

    // return 1.0;
/*

                let shade_frac = horizon_map
                    .and_then(|(angles, heights)| {
                        chunk_idx
                            .and_then(|chunk_idx| angles.get(chunk_idx))
                            .map(|&e| (e as f64, heights))
                    })
                    .and_then(|(e, heights)| {
                        chunk_idx
                            .and_then(|chunk_idx| heights.get(chunk_idx))
                            .map(|&f| (e, f as f64))
                    })
                    .map(|(angle, height)| {
                        let w = 0.1;
                        if angle != 0.0 && light_direction.x != 0.0 {
                            let deltax = height / angle;
                            let lighty = (light_direction.y / light_direction.x * deltax).abs();
                            let deltay = lighty - height;
                            let s = (deltay / deltax / w).min(1.0).max(0.0);
                            // Smoothstep
                            s * s * (3.0 - 2.0 * s)
                        } else {
                            1.0
                        }
                    })
                    .unwrap_or(1.0);
*/
    // vec2 f_horizon;
    if (light_dir.z >= 0) {
        return 0.0;
    }
    /* if (light_dir.x >= 0) {
        f_horizon = f_horizons.rg;
        // f_horizon = f_horizons.ba;
    } else {
        f_horizon = f_horizons.ba;
        // f_horizon = f_horizons.rg;
    }
    return 1.0; */
    /* bvec2 f_mode = lessThan(vec2(light_dir.x), vec2(1.0));
    f_horizon = mix(f_horizons.ba, f_horizons.rg, f_mode); */
    // f_horizon = mix(f_horizons.rg, f_horizons.ba, clamp(light_dir.x * 10000.0, 0.0, 1.0));
    vec2 f_horizon = mix(f_horizons.rg, f_horizons.ba, bvec2(light_dir.x < 0.0));
    // vec2 f_horizon = mix(f_horizons.ba, f_horizons.rg, clamp(light_dir.x * 10000.0, 0.0, 1.0));
    // f_horizon = mix(f_horizons.ba, f_horizons.rg, bvec2(lessThan(light_dir.xx, vec2(0.0))));
    /* if (f_horizon.x <= 0) {
        return 1.0;
    } */
    float angle = tan(f_horizon.x * PI_2);
    /* if (angle <= 0.0001) {
        return 1.0;
    } */
    float height = f_horizon.y * /*1300.0*//*1278.7266845703125*/view_distance.w + view_distance.z;
    const float w = 0.1;
    float deltah = height - alt;
    //if (deltah < 0.0001/* || angle < 0.0001 || abs(light_dir.x) < 0.0001*/) {
    //    return 1.0;
    /*} else */{
        float lighta = /*max*/(-light_dir.z/*, 0.0*/) / max(abs(light_dir.x), 0.0001);
        // NOTE: Ideally, deltah <= 0.0 is a sign we have an oblique horizon angle.
        float deltax = deltah / max(angle, 0.0001)/*angle*/;
        float lighty = lighta * deltax;
        float deltay = lighty - deltah + max(pos.z - alt, 0.0);
        // NOTE: the "real" deltah should always be >= 0, so we know we're only handling the 0 case with max.
        float s = mix(max(min(max(deltay, 0.0) / max(deltax, 0.0001) / w, 1.0), 0.0), 1.0, deltah <= 0);
        return max(/*0.2 + 0.8 * */(s * s * (3.0 - 2.0 * s)), MIN_LIGHT);
        /* if (lighta >= angle) {
            return 1.0;
        } else {
            return MIN_LIGHT;
        } */
        // float deltah = height - alt;
        // float deltah = max(height - alt, 0.0);
        // float lighty = abs(sun_dir.z / sun_dir.x * deltax);
        // float lighty = abs(sun_dir.z / sun_dir.x * deltax);
        // float deltay = lighty - /*pos.z*//*deltah*/(deltah + max(pos.z - alt, 0.0))/*deltah*/;
        // float s = max(min(max(deltay, 0.0) / deltax / w, 1.0), 0.0);
        // Smoothstep
        // return max(/*0.2 + 0.8 * */(s * s * (3.0 - 2.0 * s)), MIN_LIGHT);
    }
}

float horizon_at(vec3 pos, /*float time_of_day*/vec3 light_dir) {
    vec4 f_horizons = textureBicubic(t_horizon, pos_to_uv(pos.xy));
    f_horizons.xyz = /*linear_to_srgb*/(f_horizons.xyz);
    float alt = alt_at(pos.xy);
    return horizon_at2(f_horizons, alt, pos, light_dir);
}

vec2 splay(vec2 pos) {
	return pos * pow(length(pos) * 0.5, 3.0);
}

vec3 lod_norm(vec2 pos) {
	const float SAMPLE_W = 32;

	float altx0 = alt_at(pos + vec2(-1, 0) * SAMPLE_W);
	float altx1 = alt_at(pos + vec2(1, 0) * SAMPLE_W);
	float alty0 = alt_at(pos + vec2(0, -1) * SAMPLE_W);
	float alty1 = alt_at(pos + vec2(0, 1) * SAMPLE_W);
	float slope = abs(altx1 - altx0) + abs(alty0 - alty1);

	return normalize(vec3(
		(altx0 - altx1) / SAMPLE_W,
		(alty0 - alty1) / SAMPLE_W,
		SAMPLE_W / (slope + 0.00001) // Avoid NaN
	));
}

vec3 lod_pos(vec2 v_pos, vec2 focus_pos) {
	vec2 hpos = focus_pos.xy + splay(v_pos) * 1000000.0;

	// Remove spiking by "pushing" vertices towards local optima
	vec2 nhpos = hpos;
	for (int i = 0; i < 3; i ++) {
		nhpos -= lod_norm(hpos).xy * 15.0;
	}
	hpos = hpos + normalize(nhpos - hpos + 0.001) * min(length(nhpos - hpos), 32);

	return vec3(hpos, alt_at(hpos));
}

vec3 lod_col(vec2 pos) {
	//return vec3(0, 0.5, 0);
	return /*linear_to_srgb*/(textureBicubic(t_map, pos_to_uv(pos)).rgb);
		//+ (texture(t_noise, pos * 0.04 + texture(t_noise, pos * 0.005).xy * 2.0 + texture(t_noise, pos * 0.06).xy * 0.6).x - 0.5) * 0.1;
}
