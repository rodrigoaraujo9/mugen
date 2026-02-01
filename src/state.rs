use tokio::sync::{RwLock, OnceCell};
use std::sync::Arc;
use crate::audio_source::{AudioSource, SineSource};

pub struct AudioState {
    source: Arc<RwLock<Box<dyn AudioSource>>>, //defined only once for now
    volume: Arc<RwLock<f32>>, //(0.0 to 1.0)
    paused: Arc<RwLock<bool>>,
}

impl AudioState {
    fn new() -> Self {
        Self {
            source: Arc::new(RwLock::new(Box::new(SineSource::default()))),
            volume: Arc::new(RwLock::new(1.0)),
            paused: Arc::new(RwLock::new(false)),
        }
    }

    pub fn get_source(&self) -> Arc<RwLock<Box<dyn AudioSource>>> {
        Arc::clone(&self.source)
    }

    pub async fn set_source(&self, new_source: Box<dyn AudioSource>) {
        let mut source = self.source.write().await;
        *source = new_source;
    }

    pub async fn get_volume(&self) -> f32 {
        *self.volume.read().await
    }

    pub async fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        let mut volume = self.volume.write().await;
        *volume = clamped;
    }

    pub async fn is_paused(&self) -> bool {
        *self.paused.read().await
    }

    pub async fn set_paused(&self, paused: bool) {
        let mut p = self.paused.write().await;
        *p = paused;
    }

    pub async fn toggle_pause(&self) -> bool {
        let mut p = self.paused.write().await;
        *p = !*p;
        *p
    }
}

static AUDIO_STATE: OnceCell<AudioState> = OnceCell::const_new();

async fn get_audio_state() -> &'static AudioState {
    AUDIO_STATE.get_or_init(|| async { AudioState::new() }).await
}

pub async fn get_source() -> Arc<RwLock<Box<dyn AudioSource>>> {
    get_audio_state().await.get_source()
}

pub async fn set_source(source: Box<dyn AudioSource>) {
    get_audio_state().await.set_source(source).await;
}

pub async fn get_volume() -> f32 {
    get_audio_state().await.get_volume().await
}

pub async fn set_volume(volume: f32) {
    get_audio_state().await.set_volume(volume).await;
}

pub async fn is_paused() -> bool {
    get_audio_state().await.is_paused().await
}

pub async fn set_paused(paused: bool) {
    get_audio_state().await.set_paused(paused).await;
}

pub async fn toggle_pause() -> bool {
    get_audio_state().await.toggle_pause().await
}
