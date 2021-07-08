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

static const uint MAX_VERTEX_COUNT = 64;
static const uint MAX_TRIANGLE_COUNT = 126;

// constant_buffer
// vertex_buffer
// meshlet_buffer
// triangle_indices
// vertex_indices

#define ROOT_SIG "CBV(b0), \
                  SRV(t0), \
                  SRV(t1), \
                  SRV(t2), \
                  SRV(t3),"

struct Constants
{
    float4x4 world_view_proj;
};

struct Vertex
{
    float3 position;
};

struct VertexOut
{
    float4 position_hs   : SV_Position;
    uint   meshlet_index : COLOR0;
};

struct Meshlet
{
    uint vertex_count;
    uint triangles_offset;
    uint triangle_count;
    uint vertices_offset; 
};

ConstantBuffer<Constants> Globals             : register(b0);
StructuredBuffer<Vertex>  Vertices            : register(t0);
StructuredBuffer<Meshlet> Meshlets            : register(t1);
StructuredBuffer<uint> TriangleIndices                   : register(t2);
StructuredBuffer<uint> VertexIndices                     : register(t3);

[RootSignature(ROOT_SIG)]
[NumThreads(128, 1, 1)]
[OutputTopology("triangle")]
void main(
    uint group_thread_id : SV_GroupThreadID,
    uint group_id : SV_GroupID,
    out indices uint3 tris[MAX_TRIANGLE_COUNT],
    out vertices VertexOut verts[MAX_VERTEX_COUNT]
)
{
    Meshlet m = Meshlets[group_id];

    SetMeshOutputCounts(m.vertex_count, m.triangle_count);

    if (group_thread_id < m.triangle_count)
    {
        uint3 local_indices = uint3(
            TriangleIndices[m.triangles_offset + group_thread_id * 3],
            TriangleIndices[m.triangles_offset + group_thread_id * 3 + 1],
            TriangleIndices[m.triangles_offset + group_thread_id * 3 + 2]
        );

        tris[group_thread_id] = local_indices;
    }

    if (group_thread_id < m.vertex_count)
    {
        uint vertex_index = VertexIndices[m.vertices_offset + group_thread_id];
        Vertex vert = Vertices[vertex_index];
        VertexOut vout = (VertexOut)0;

        vout.position_hs = mul(Globals.world_view_proj, float4(vert.position, 1));
        vout.meshlet_index = group_id;

        verts[group_thread_id] = vout;
    }
}
