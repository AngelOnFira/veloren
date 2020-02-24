#include <random.glsl>

uniform sampler2D t_map;

vec2 pos_to_uv(vec2 pos) {
	vec2 uv_pos = pos / 32768.0;
	return vec2(uv_pos.x, 1.0 - uv_pos.y);
}

float alt_at(vec2 pos) {
	return texture(t_map, pos_to_uv(pos)).a * (1300.0) + 140.0;

	return 0.0
		+ pow(texture(t_noise, pos * 0.00005).x * 1.4, 3.0) * 1000.0
		+ texture(t_noise, pos * 0.001).x * 100.0
		+ texture(t_noise, pos * 0.003).x * 30.0;
}

vec2 splay(vec2 pos, float e) {
	return pos * pow(length(pos), e);
}

float splay_scale(vec2 pos, float e) {
	return distance(splay(pos, e), splay(pos + 0.001, e)) * 1000000.0;
}

vec3 lod_norm(vec2 pos) {
	const float SAMPLE_W = 16;

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
	vec2 hpos = focus_pos.xy + splay(v_pos, 4.0) * 1000000.0;

	// Remove spiking by "pushing" vertices towards local optima
	vec2 nhpos = hpos;
	for (int i = 0; i < 5; i ++) {
		nhpos -= lod_norm(hpos).xy * 5.0;
	}
	hpos = nhpos;

	return vec3(hpos, alt_at(hpos));
}

vec3 lod_col(vec2 pos) {
	//return vec3(0, 1, 0);
	return texture(t_map, pos_to_uv(pos)).rgb;
		//+ (texture(t_noise, pos * 0.04 + texture(t_noise, pos * 0.005).xy * 2.0 + texture(t_noise, pos * 0.06).xy * 0.6).x - 0.5) * 0.1;

	vec3 warmth = mix(
		vec3(0.05, 0.4, 0.1),
		vec3(0.5, 0.4, 0.0),
		(texture(t_noise, pos * 0.0002).x - 0.5) * 2.0 + 0.5
	);

	vec3 color = mix(
		warmth,
		vec3(0.3, 0.3, 0.4),
		alt_at(pos) / 1200.0
	);

	return color;
}
