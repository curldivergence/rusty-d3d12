//*********************************************************
//
// Copyright (c) Microsoft. All rights reserved.
// This code is licensed under the MIT License (MIT).
// THIS CODE IS PROVIDED *AS IS* WITHOUT WARRANTY OF
// ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING ANY
// IMPLIED WARRANTIES OF FITNESS FOR A PARTICULAR
// PURPOSE, MERCHANTABILITY, OR NON-INFRINGEMENT.
//
//*********************************************************


struct VertexOut
{
    float4 PositionHS   : SV_Position;
    uint   MeshletIndex : COLOR0;
};



float4 main(VertexOut input) : SV_TARGET
{
    const float3 palette[16] = {
        float3(0.85, 0.64, 0.48),
        float3(0.27, 0.96, 0.06),
        float3(0.17, 0.87, 0.38),
        float3(0.96, 0.12, 0.35),
        float3(0.61, 0.17, 0.95),
        float3(0.16, 0.49, 0.77),
        float3(0.66, 0.54, 0.77),
        float3(0.19, 1.00, 0.36),
        float3(0.79, 0.41, 0.72),
        float3(0.58, 0.09, 0.46),
        float3(0.57, 0.35, 0.24),
        float3(0.66, 0.94, 0.78),
        float3(0.93, 0.29, 0.05),
        float3(0.14, 0.09, 0.52),
        float3(0.68, 0.65, 0.48),
        float3(0.50, 0.45, 0.83)
    };
    float3 color = palette[input.MeshletIndex % 16];

    return float4(color, 1);
}
