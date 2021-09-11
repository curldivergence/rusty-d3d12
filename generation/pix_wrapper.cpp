#include "pix_wrapper.h"

#define USE_PIX
#include "DXProgrammableCapture.h"
#include "pix3.h"

static IDXGraphicsAnalysis *gs_AnalysisInterface = nullptr;
static HRESULT gs_InitResult = E_FAIL;

void pix_init_analysis()
{
    gs_InitResult = DXGIGetDebugInterface1(0, IID_PPV_ARGS(&gs_AnalysisInterface));
}

void pix_shutdown_analysis()
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        gs_AnalysisInterface->Release();
}

void pix_begin_capture()
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        gs_AnalysisInterface->BeginCapture();
}

void pix_end_capture()
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        gs_AnalysisInterface->EndCapture();
}

void pix_begin_event_cmd_list(ID3D12GraphicsCommandList6 *command_list, UINT64 color, const char *marker)
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        PIXBeginEvent(command_list, color, "%s", marker);
}

void pix_end_event_cmd_list(ID3D12GraphicsCommandList6 *command_list)
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        PIXEndEvent(command_list);
}

void pix_begin_event_cmd_queue(ID3D12CommandQueue *command_queue, UINT64 color, const char *marker)
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        PIXBeginEvent(command_queue, color, "%s", marker);
}

void pix_end_event_cmd_queue(ID3D12CommandQueue *command_queue)
{
    if (gs_AnalysisInterface && gs_InitResult == S_OK)
        PIXEndEvent(command_queue);
}