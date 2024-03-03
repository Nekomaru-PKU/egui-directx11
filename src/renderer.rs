
use std::mem;

use egui::Pos2;
use egui::Rgba;
use egui::ClippedPrimitive;
use egui::epaint::Primitive;
use egui::epaint::Vertex;

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

impl Renderer {
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

    pub fn render(
        &mut self,
        device_context: &ID3D11DeviceContext,
        render_target: &ID3D11RenderTargetView,
        egui_ctx: &egui::Context,
        egui_output: egui::FullOutput,
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