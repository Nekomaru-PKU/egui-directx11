
use std::mem;

use egui::Pos2;
use egui::Rgba;
use egui::ClippedPrimitive;
use egui::epaint::ClippedShape;
use egui::epaint::Primitive;
use egui::epaint::Vertex;
use egui::epaint::textures::TexturesDelta;

#[repr(C)]
struct VertexInput {
    pos: Pos2,
    uv: Pos2,
    color: Rgba,
}

use windows::core::Result;
use windows::core::Interface;
use windows::Win32::Foundation::BOOL;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use crate::texture::TexturePool;

/// The core of this crate. You can set up a renderer via [`Renderer::new`]
/// and render the output from `egui` with [`Renderer::render`].
pub struct Renderer {
    device: ID3D11Device,

    input_layout: ID3D11InputLayout,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    rasterizer_state: ID3D11RasterizerState,
    sampler_state: ID3D11SamplerState,
    blend_state: ID3D11BlendState,

    texture_pool: TexturePool,
}

/// Part of [`egui::FullOutput`] that is consumed by [`Renderer::render`].
/// 
/// Call to [`egui::Context::run`] or [`egui::Context::end_frame`] yields a
/// [`egui::FullOutput`]. The platform integration (for example `egui_winit`)
/// consumes [`egui::FullOutput::platform_output`] and [`egui::FullOutput::viewport_output`],
/// and the renderer consumes the rest.
/// 
/// To conveniently split a [`egui::FullOutput`] into a [`RendererOutput`] and
/// outputs for the platform integration, use [`split_output`].
#[allow(missing_docs)]
pub struct RendererOutput {
    pub textures_delta: TexturesDelta,
    pub shapes: Vec<ClippedShape>,
    pub pixels_per_point: f32,
}

/// Convenience method to split a [`egui::FullOutput`] into the [`RendererOutput`]
/// part and other parts for platform integration.
pub fn split_output(full_output: egui::FullOutput) -> (
    RendererOutput,
    egui::PlatformOutput,
    egui::ViewportIdMap<egui::ViewportOutput>
) {(
    RendererOutput {
        textures_delta: full_output.textures_delta,
        shapes: full_output.shapes,
        pixels_per_point: full_output.pixels_per_point
    },
    full_output.platform_output,
    full_output.viewport_output,
)}

impl Renderer {
    /// Create a [`Renderer`] using the provided D3D11 device. The [`Renderer`]
    /// holds various D3D11 resources and states derived from the device.
    pub fn new(device: &ID3D11Device)-> Result<Self> {
        let input_layout = crate::unwrap(|ret_| unsafe {
            device.CreateInputLayout(
                &Self::INPUT_ELEMENTS_DESC,
                Self::VS_BLOB,
                ret_)
        })?;
        let vertex_shader = crate::unwrap(|ret_| unsafe {
            device.CreateVertexShader(Self::VS_BLOB, None, ret_)
        })?;
        let pixel_shader = crate::unwrap(|ret_| unsafe {
            device.CreatePixelShader(Self::PS_BLOB, None, ret_)
        })?;
        let rasterizer_state = crate::unwrap(|ret_| unsafe {
            device.CreateRasterizerState(&Self::RASTERIZER_DESC, ret_)
        })?;
        let sampler_state = crate::unwrap(|ret_| unsafe {
            device.CreateSamplerState(&Self::SAMPLER_DESC, ret_)
        })?;
        let blend_state = crate::unwrap(|ret_| unsafe {
            device.CreateBlendState(&Self::BLEND_DESC, ret_)
        })?;

        let texture_pool = TexturePool::new(device);
        Ok(Self {
            device: device.clone(),

            input_layout,
            vertex_shader,
            pixel_shader,
            rasterizer_state,
            sampler_state,
            blend_state,

            texture_pool,
        })
    }

    /// Render the output of `egui` to the provided render target using the
    /// provided device context. The render target should use a linear color
    /// space (e.g. `DXGI_FORMAT_R8G8B8A8_UNORM_SRGB`) for proper results.
    /// 
    /// The `scale_factor` should be the scale factor of your window and not
    /// confused with [`egui::Context::zoom_factor`]. If you are using `winit`,
    /// the `scale_factor` can be aquired using `Window::scale_factor`.
    /// 
    /// Note that this function does not maintain the current state of the
    /// D3D11 graphics pipeline. Particularly, it calls
    /// [`ID3D11DeviceContext::ClearState`](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-clearstate)
    /// before returning, so it is all *your* responsibility to backup the
    /// current pipeline state and restore it afterwards if your rendering
    /// pipeline depends on it.
    /// 
    /// See the [`egui-demo`](https://github.com/Nekomaru-PKU/egui-directx11/blob/main/examples/egui-demo/src/main.rs)
    /// example for code examples.
    pub fn render(
        &mut self,
        device_context: &ID3D11DeviceContext,
        render_target: &ID3D11RenderTargetView,
        egui_ctx: &egui::Context,
        egui_output: RendererOutput,
        scale_factor: f32,
    )-> Result<()> {
        self.texture_pool.update(device_context, egui_output.textures_delta)?;
        if egui_output.shapes.is_empty() { return Ok(()); }

        let ctx = device_context;
        let frame_size = Self::get_render_target_size(render_target)?;
        let frame_size_scaled = (
            frame_size.0 as f32 / scale_factor,
            frame_size.1 as f32 / scale_factor);
        let zoom_factor = egui_ctx.zoom_factor();

        unsafe {
            ctx.ClearState();
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetInputLayout(&self.input_layout);
            ctx.VSSetShader(&self.vertex_shader, Some(&[]));
            ctx.PSSetShader(&self.pixel_shader, Some(&[]));
            ctx.RSSetState(&self.rasterizer_state);
            ctx.RSSetViewports(Some(&[D3D11_VIEWPORT {
                TopLeftX: 0.,
                TopLeftY: 0.,
                Width : frame_size.0 as _,
                Height: frame_size.1 as _,
                MinDepth: 0.,
                MaxDepth: 1.,
            }]));
            ctx.PSSetSamplers(0, Some(&[Some(self.sampler_state.clone())]));
            ctx.OMSetRenderTargets(Some(&[Some(render_target.clone())]), None);
            ctx.OMSetBlendState(&self.blend_state, Some(&[0.; 4]), u32::MAX);
        }
        for (vtx, idx, tex, clip_rect) in egui_ctx
            .tessellate(egui_output.shapes, egui_output.pixels_per_point)
            .into_iter()
            .filter_map(|ClippedPrimitive { primitive, clip_rect }| match primitive {
                Primitive::Mesh(mesh) => Some((mesh, clip_rect)),
                Primitive::Callback(..) => {
                    log::warn!("paint callbacks are not yet supported.");
                    None
                }
            })
            .filter_map(|(mesh, clip_rect)| {
                if mesh.indices.is_empty() { return None; }
                if mesh.indices.len() % 3 != 0 {
                    log::warn!("egui wants to draw a incomplete triangle. this request will be ignored.");
                    return None;
                }
                let clip_rect_scaled = clip_rect * scale_factor * zoom_factor;
                let vtx = mesh.vertices.into_iter()
                    .map(|Vertex { pos, uv, color }| VertexInput {
                        pos: Pos2::new(
                            pos.x * zoom_factor / frame_size_scaled.0 * 2.0 - 1.0,
                            1.0 - pos.y * zoom_factor / frame_size_scaled.1 * 2.0),
                        uv,
                        color: color.into(),
                    })
                    .collect::<Vec<_>>();
                Some((vtx, mesh.indices, mesh.texture_id, clip_rect_scaled))
            }) {
            let vb = Self::create_index_buffer(&self.device, &idx)?;
            let ib = Self::create_vertex_buffer(&self.device, &vtx)?;
            unsafe {
                ctx.IASetVertexBuffers(
                    0,
                    1,
                    Some(&Some(ib)),
                    Some(&(mem::size_of::<VertexInput>() as _)),
                    Some(&0),
                );
                ctx.IASetIndexBuffer(
                    &vb,
                    DXGI_FORMAT_R32_UINT,
                    0);
                ctx.RSSetScissorRects(Some(&[RECT {
                    left  : clip_rect.left() as _,
                    top   : clip_rect.top() as _,
                    right : clip_rect.right() as _,
                    bottom: clip_rect.bottom() as _,
                }]));
                if let Some(tex) = self.texture_pool.get_srv(tex) {
                    ctx.PSSetShaderResources(0, Some(&[Some(tex)]));
                } else {
                    log::warn!("egui wants to sample a non-existing texture {:?}. this request will be ignored.", tex);
                };
                ctx.DrawIndexed(idx.len() as _, 0, 0);
            }
        };
        unsafe { ctx.ClearState() };
        Ok(())
    }
}

impl Renderer {
    const VS_BLOB: &'static [u8] = include_bytes!("../shaders/egui_vs.bin");
    const PS_BLOB: &'static [u8] = include_bytes!("../shaders/egui_ps.bin");

    const INPUT_ELEMENTS_DESC: [D3D11_INPUT_ELEMENT_DESC; 3] = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    const RASTERIZER_DESC: D3D11_RASTERIZER_DESC = D3D11_RASTERIZER_DESC {
        FillMode: D3D11_FILL_SOLID,
        CullMode: D3D11_CULL_NONE,
        FrontCounterClockwise: BOOL(0),
        DepthBias: 0,
        DepthBiasClamp: 0.,
        SlopeScaledDepthBias: 0.,
        DepthClipEnable: BOOL(0),
        ScissorEnable: BOOL(1),
        MultisampleEnable: BOOL(0),
        AntialiasedLineEnable: BOOL(0),
    };

    const SAMPLER_DESC: D3D11_SAMPLER_DESC = D3D11_SAMPLER_DESC {
        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
        AddressU: D3D11_TEXTURE_ADDRESS_BORDER,
        AddressV: D3D11_TEXTURE_ADDRESS_BORDER,
        AddressW: D3D11_TEXTURE_ADDRESS_BORDER,
        ComparisonFunc: D3D11_COMPARISON_ALWAYS,
        BorderColor: [1., 1., 1., 1.],
        .. crate::zeroed()
    };

    const BLEND_DESC: D3D11_BLEND_DESC = D3D11_BLEND_DESC {
        RenderTarget: [
            D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: BOOL(1),
                SrcBlend: D3D11_BLEND_SRC_ALPHA,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as _,
            },
            crate::zeroed(),
            crate::zeroed(),
            crate::zeroed(),
            crate::zeroed(),
            crate::zeroed(),
            crate::zeroed(),
            crate::zeroed(),
        ],..crate::zeroed()
    };
}

impl Renderer {
    fn create_vertex_buffer(
        device: &ID3D11Device,
        data: &[VertexInput],
    )-> Result<ID3D11Buffer> {
        crate::unwrap(|ret_| unsafe {
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: mem::size_of_val(data) as _,
                    Usage: D3D11_USAGE_IMMUTABLE,
                    BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as _,
                    ..D3D11_BUFFER_DESC::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.as_ptr() as _,
                    ..D3D11_SUBRESOURCE_DATA::default()
                }),
                ret_)
        })
    }

    fn create_index_buffer(
        device: &ID3D11Device,
        data: &[u32],
    )-> Result<ID3D11Buffer> {
        crate::unwrap(|ret_| unsafe {
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: mem::size_of_val(data) as _,
                    Usage: D3D11_USAGE_IMMUTABLE,
                    BindFlags: D3D11_BIND_INDEX_BUFFER.0 as _,
                    ..D3D11_BUFFER_DESC::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.as_ptr() as _,
                    ..D3D11_SUBRESOURCE_DATA::default()
                }),
                ret_)
        })
    }

    fn get_render_target_size(rtv: &ID3D11RenderTargetView) -> Result<(u32, u32)> {
        let tex = unsafe { rtv.GetResource() }?.cast::<ID3D11Texture2D>()?;
        let mut desc = crate::zeroed();
        unsafe { tex.GetDesc(&mut desc) };
        Ok((desc.Width, desc.Height))
    }
}