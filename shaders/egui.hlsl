void vs_egui(
    in const float2 i_pos  : POSITION,
    in const float2 i_uv   : TEXCOORD,
    in const float4 i_color: COLOR,
    out      float4 o_pos  : SV_POSITION,
    out      float2 o_uv   : TEXCOORD,
    out      float4 o_color: COLOR) {
    o_pos   = float4(i_pos, 0.0, 1.0);
    o_uv    = i_uv;
    o_color = i_color;
}

Texture2D<float4> g_texture: register(t0);
SamplerState      g_sampler: register(s0);

float3 linear_to_gamma(float3 color) {
    bool3 cutoff = color < 0.0031308f;
    float3 lower = color * 12.92f;
    float3 higher = 1.055f * pow(max(color, 0.0f), 1.0f / 2.4f) - 0.055f;
    return cutoff ? lower : higher;
}

float4 ps_egui(
    in const float4 i_pos  : SV_POSITION,
    in const float2 i_uv   : TEXCOORD,
    in const float4 i_color: COLOR): SV_TARGET {
    return i_color * g_texture.Sample(g_sampler, i_uv);
}

float4 ps_egui_gamma(
    in const float4 i_pos  : SV_POSITION,
    in const float2 i_uv   : TEXCOORD,
    in const float4 i_color: COLOR): SV_TARGET {
    float4 color = i_color * g_texture.Sample(g_sampler, i_uv);
    return float4(linear_to_gamma(color.rgb), color.a);
}
