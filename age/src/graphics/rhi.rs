use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock, RwLock,
    },
    thread::JoinHandle,
};

use crate::{Error, RenderProxy, Renderer};

use super::renderer;

static RHI: OnceLock<Rhi> = OnceLock::new();

pub struct Rhi {
    inner: Arc<RwLock<RhiInner>>,
    stop: Arc<AtomicBool>,
}

struct RhiInner {
    thread: Option<JoinHandle<()>>,
    proxy: Option<RenderProxy>,
    resource_ids: Vec<Option<RendererResource>>,
    free_ids: VecDeque<usize>,
}

impl Default for Rhi {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RhiInner {
                thread: None,
                proxy: None,
                resource_ids: Vec::new(),
                free_ids: VecDeque::new(),
            })),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Rhi {
    pub(crate) fn get() -> &'static Rhi {
        RHI.get_or_init(Self::default)
    }

    pub(crate) fn init(&self) -> Result<(), Error> {
        let renderer = Renderer::new(self.stop.clone())?;
        let render_proxy = renderer.create_render_proxy();

        let thread = std::thread::Builder::new()
            .name("renderer".to_string())
            .spawn(|| {
                if let Err(err) = renderer::main_thread(renderer) {
                    eprintln!("{err}");
                }
            })?;

        {
            let mut inner = self.inner_mut();
            inner.thread = Some(thread);
            inner.proxy = Some(render_proxy);
        }

        Ok(())
    }

    pub(crate) fn deinit(&self) -> Result<(), Error> {
        self.stop.store(true, Ordering::Relaxed);

        let thread = {
            let mut inner = self.inner_mut();
            inner.thread.take()
        };

        if let Some(thread) = thread {
            if let Err(msg) = thread.join() {
                return Err(Error::new(format!("render thread panicked: {:?}", msg)));
            }
        }

        Ok(())
    }

    pub(crate) fn get_render_proxy(&self) -> RenderProxy {
        self.inner()
            .proxy
            .as_ref()
            .expect("rhi has not been initialised")
            .clone()
    }

    pub(crate) fn reserve_renderer_id(&self, resource: RendererResource) -> RendererId {
        let mut inner = self.inner_mut();
        let index = match inner.free_ids.pop_front() {
            Some(index) => index,
            None => {
                let len = inner.resource_ids.len();
                let new_len = len + 1;
                inner.resource_ids.resize_with(new_len, || None);
                len
            }
        };

        inner.resource_ids[index] = Some(resource);

        // Top 8 bits are resource type, bottom 24 index.
        RendererId((resource as u32) << 24 | (index as u32))
    }

    pub(crate) fn release_renderer_id(&self, id: RendererId) {
        let index = (id.0 & 0xFFFFFF) as usize;
        let mut inner = self.inner_mut();
        inner.resource_ids[index] = None;
        inner.free_ids.push_back(index);
    }

    fn inner(&self) -> std::sync::RwLockReadGuard<'_, RhiInner> {
        self.inner.read().expect("failed to acquire write lock")
    }

    fn inner_mut(&self) -> std::sync::RwLockWriteGuard<'_, RhiInner> {
        self.inner.write().expect("failed to acquire write lock")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RendererId(u32);

impl RendererId {
    pub const INVALID: Self = Self(0xFFFFFFFF);

    pub(crate) fn split(&self) -> (u8, u32) {
        let ty = self.0 >> 24;
        let index = self.0 & 0xFFFFFF;
        (ty as u8, index)
    }
}

impl Default for RendererId {
    fn default() -> Self {
        Self::INVALID
    }
}

impl std::fmt::Debug for RendererId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (ty, index) = self.split();
        let resource = RendererResource::try_from(ty).map_err(|_| std::fmt::Error)?;
        f.debug_tuple("RendererId")
            .field(&resource)
            .field(&index)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererResource {
    Backbuffer = 1,
}

impl TryFrom<u8> for RendererResource {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RendererResource::Backbuffer),
            _ => Err(Error::new("invalid renderer resource")),
        }
    }
}
