#include <windows.h>
#include <dxgi1_3.h>
#include <d3d12.h>

#ifdef __cplusplus
extern "C"
{
#endif
    void pix_init_analysis();
    void pix_shutdown_analysis();
    void pix_begin_capture();
    void pix_end_capture();
    void pix_begin_event_cmd_list(ID3D12GraphicsCommandList6 *command_list, UINT64 color, const char *marker);
    void pix_end_event_cmd_list(ID3D12GraphicsCommandList6 *command_list);
    void pix_begin_event_cmd_queue(ID3D12CommandQueue *command_queue, UINT64 color, const char *marker);
    void pix_end_event_cmd_queue(ID3D12CommandQueue *command_queue);

#ifdef __cplusplus
}
#endif