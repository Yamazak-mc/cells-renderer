use crate::{AppConfigs, World};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

mod app_impl;
use app_impl::AppImpl;

pub struct App<'window, W> {
    state: AppState<'window, W>,
}

enum AppState<'window, W> {
    Ready(Option<(AppConfigs, W)>),
    Running(AppImpl<'window, W>),
}

impl<'window, W> AppState<'window, W> {
    fn init<F>(&mut self, initializer: F)
    where
        F: FnOnce(AppConfigs, W) -> AppImpl<'window, W>,
    {
        let Self::Ready(data) = self else {
            panic!("AppState::init called on AppState::Running");
        };
        let (configs, world) = data.take().unwrap();

        let app = initializer(configs, world);
        *self = Self::Running(app);
    }

    fn unwrap_running_mut(&mut self) -> &mut AppImpl<'window, W> {
        match self {
            Self::Running(app) => app,
            _ => panic!("unwrap_running_mut called on AppState::Ready"),
        }
    }
}

impl<W: World> App<'_, W> {
    #[inline]
    pub fn new(configs: AppConfigs, world: W) -> Self {
        Self {
            state: AppState::Ready(Some((configs, world))),
        }
    }

    #[inline]
    pub fn run(mut self) -> anyhow::Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

impl<W: World> ApplicationHandler for App<'_, W> {
    #[inline]
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state.init(|configs, world| {
            futures::executor::block_on(AppImpl::new(configs, world, event_loop)).unwrap()
        });
    }

    #[inline]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.state
            .unwrap_running_mut()
            .window_event(event_loop, window_id, event)
            .unwrap();
    }
}
