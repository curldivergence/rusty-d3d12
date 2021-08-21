struct SceneConstantBuffer
{
    float4 offset;
    float4 color;
};

ConstantBuffer<SceneConstantBuffer> scene_cbuffer: register(b0);

struct PSInput
{
    float4 position : SV_POSITION;
    float4 color : COLOR;
};

PSInput VShader(float4 position : POSITION)
{
    PSInput result;

    result.position = position + scene_cbuffer.offset;
    result.color = scene_cbuffer.color;
    return result;
}

float4 PShader(PSInput input) : SV_TARGET
{
    return input.color;
}
